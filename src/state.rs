use core::ops::{Mul, Range};
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

pub struct Groups(Vec<Range<usize>>);

impl Groups {
    pub fn one<A>() -> Self
    where
        A: WOtsPlus,
    {
        let (l1, l2) = State::<A>::lengths();
        Groups(
            (0..(l1 + l2))
                .map(|_| 0..(A::WinternitzMinusOne::USIZE + 1))
                .collect(),
        )
    }

    pub fn inverse<A>(self) -> Self
    where
        A: WOtsPlus,
    {
        Groups(
            self.0
                .into_iter()
                .map(|Range { start: _, end: e }| e..(A::WinternitzMinusOne::USIZE + 1))
                .collect(),
        )
    }

    fn checksum<A>(self) -> Self
    where
        A: WOtsPlus,
    {
        let (l1, l2) = State::<A>::lengths();
        let sum = self.0[0..l1].iter().fold(
            0,
            |sum,
             &Range {
                 start: _,
                 end: ref e,
             }| { sum + A::WinternitzMinusOne::USIZE - e.clone() },
        );
        let (s, _) = (0..l2).fold((self, sum), |(s, sum), _| {
            (s.add((sum & 0xf) as u8), sum / 0x10)
        });
        s
    }

    fn add(self, v: u8) -> Self {
        let mut s = self;
        s.0.push(0..(v as usize));
        s
    }

    pub fn message<A>(message: GenericArray<u8, A::MessageSize>) -> Self
    where
        A: WOtsPlus,
    {
        match A::WinternitzMinusOne::USIZE {
            15 => message
                .into_iter()
                .fold(Groups(Vec::new()), |g, x| g.add(x / 0x10).add(x & 0xf))
                .checksum::<A>(),
            _ => unimplemented!(),
        }
    }
}

impl<A> Mul<Groups> for &State<A>
where
    A: WOtsPlus,
{
    type Output = State<A>;

    fn mul(self, rhs: Groups) -> State<A> {
        use digest::generic_array::sequence::GenericSequence;

        State {
            randomization: self.randomization.clone(),
            data: self
                .data
                .iter()
                .zip(rhs.0)
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
