use core::{
    fmt,
    ops::{Mul, Range},
    marker::PhantomData,
};
use digest::{
    generic_array::{GenericArray, ArrayLength, typenum::Unsigned},
    Digest,
};

pub trait WOtsPlus {
    type BlockLength: ArrayLength<u8>;
    type MessageSize: ArrayLength<u8>;
    type WinternitzMinusOne: ArrayLength<GenericArray<u8, Self::BlockLength>>;
    type Digest: Digest<OutputSize = Self::BlockLength>;
}

impl<N, M, W, D, R> WOtsPlus for (N, M, W, D, R)
where
    N: ArrayLength<u8>,
    M: ArrayLength<u8>,
    W: ArrayLength<GenericArray<u8, N>>,
    D: Digest<OutputSize = N>,
{
    type BlockLength = N;
    type MessageSize = M;
    type WinternitzMinusOne = W;
    type Digest = D;
}

#[derive(Clone, Eq, PartialEq)]
pub struct State<A>
where
    A: WOtsPlus,
{
    randomization: GenericArray<GenericArray<u8, A::BlockLength>, A::WinternitzMinusOne>,
    data: Vec<GenericArray<u8, A::BlockLength>>,
}

impl<A> State<A>
where
    A: WOtsPlus,
{
    pub fn lengths() -> (usize, usize) {
        let m = A::MessageSize::U64 as f64;
        let w = A::WinternitzMinusOne::U64 as f64;
        let l1 = (m * 8.0 / (w + 1.0).log2()).ceil();
        let l2 = 1.0 + ((l1 * w).log2() / w.log2()).floor();
        (l1 as usize, l2 as usize)
    }

    pub fn new(
        randomization: GenericArray<GenericArray<u8, A::BlockLength>, A::WinternitzMinusOne>,
        data: Vec<GenericArray<u8, A::BlockLength>>,
    ) -> Self {
        let (l1, l2) = Self::lengths();
        assert_eq!(l1 + l2, data.len());
        State {
            randomization: randomization,
            data: data,
        }
    }

    pub fn randomization(
        &self,
    ) -> &GenericArray<GenericArray<u8, A::BlockLength>, A::WinternitzMinusOne> {
        &self.randomization
    }

    pub fn data(&self) -> &[GenericArray<u8, A::BlockLength>] {
        self.data.as_ref()
    }

    pub fn project(self) -> Vec<GenericArray<u8, A::BlockLength>> {
        self.data
    }
}

impl<A> fmt::Debug for State<A>
where
    A: WOtsPlus,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct ByteArray<L>(GenericArray<u8, L>)
        where
            L: ArrayLength<u8>;

        impl<L> fmt::Debug for ByteArray<L>
        where
            L: ArrayLength<u8>,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", hex::encode(&self.0))
            }
        }

        f.debug_list()
            .entries(self.randomization.iter().cloned().map(ByteArray))
            .entries(self.data.iter().cloned().map(ByteArray))
            .finish()
    }
}

pub struct Message<A>
where
    A: WOtsPlus,
{
    ranges: Vec<Range<usize>>,
    phantom_data: PhantomData<A>,
}

impl<A> Message<A>
where
    A: WOtsPlus,
{
    fn empty() -> Self
    where
        A: WOtsPlus,
    {
        let (l1, l2) = State::<A>::lengths();
        let data = Vec::with_capacity(l1 + l2);
        Message {
            ranges: data,
            phantom_data: PhantomData,
        }
    }

    pub fn infinity() -> Self
    where
        A: WOtsPlus,
    {
        let (l1, l2) = State::<A>::lengths();
        Message {
            ranges: (0..(l1 + l2))
                .map(|_| 0..(A::WinternitzMinusOne::USIZE + 1))
                .collect(),
            phantom_data: PhantomData,
        }
    }

    pub fn inverse(self) -> Self {
        Message {
            ranges: self
                .ranges
                .into_iter()
                .map(|Range { start: _, end: e }| e..(A::WinternitzMinusOne::USIZE + 1))
                .collect(),
            phantom_data: PhantomData,
        }
    }

    fn add(self, v: u8) -> Self {
        let mut s = self;
        s.ranges.push(0..(v as usize));
        s
    }

    fn add_many(self, buffer: &[u8], count: usize) -> Self {
        let Message {
            ranges: ranges,
            phantom_data: _,
        } = match A::WinternitzMinusOne::USIZE {
            0x0f => buffer
                .iter()
                .fold(Message::<A>::empty(), |g, &x| g.add(x / 0x10).add(x & 0xf)),
            0xff => buffer.iter().fold(Message::<A>::empty(), |g, &x| g.add(x)),
            _ => unimplemented!(),
        };

        assert!(ranges.len() >= count);
        let base = ranges.len() - count;
        let mut s = self;
        s.ranges.extend_from_slice(&ranges[base..]);
        s
    }

    fn checksum(self) -> Self {
        use core::mem;
        use byteorder::{ByteOrder, BigEndian};

        let (l1, l2) = State::<A>::lengths();

        // works only if `l2` fit in u64, e.g. 3 * `size of group` <= 8
        assert!(l2 * A::MessageSize::USIZE / l1 <= mem::size_of::<u64>());

        let sum = self.ranges[0..l1].iter().fold(
            0,
            |sum,
             &Range {
                 start: _,
                 end: ref e,
             }| { sum + ((A::WinternitzMinusOne::USIZE - e.clone()) as u64) },
        );
        let mut buffer = [0; 8];
        BigEndian::write_u64(&mut buffer, sum);
        self.add_many(buffer.as_ref(), l2)
    }

    pub fn message(message: GenericArray<u8, A::MessageSize>) -> Self {
        let (l1, _) = State::<A>::lengths();
        Message::empty().add_many(message.as_ref(), l1).checksum()
    }
}

impl<A> Mul<Message<A>> for &State<A>
where
    A: WOtsPlus,
{
    type Output = State<A>;

    fn mul(self, rhs: Message<A>) -> State<A> {
        use digest::generic_array::sequence::GenericSequence;

        State {
            randomization: self.randomization.clone(),
            data: self
                .data
                .iter()
                .zip(rhs.ranges)
                .map(|(block, range)| {
                    self.randomization[range]
                        .iter()
                        .fold(block.clone(), |b, a| {
                            let v = GenericArray::<u8, A::BlockLength>::generate(|i| a[i] ^ b[i]);
                            A::Digest::new().chain(v).result()
                        })
                })
                .collect(),
        }
    }
}
