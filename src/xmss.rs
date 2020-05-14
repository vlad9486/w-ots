use super::state::{WOtsPlus, State, Message};

use digest::generic_array::GenericArray;

// TODO: XMSS tree

#[derive(Clone)]
pub struct SecretKey<A>(State<A>)
where
    A: WOtsPlus;

impl<A> SecretKey<A>
where
    A: WOtsPlus,
{
    pub fn new(
        randomization: GenericArray<GenericArray<u8, A::BlockLength>, A::WinternitzMinusOne>,
        data: Vec<GenericArray<u8, A::BlockLength>>,
    ) -> Self {
        SecretKey(State::new(randomization, data))
    }

    pub fn randomization(
        &self,
    ) -> &GenericArray<GenericArray<u8, A::BlockLength>, A::WinternitzMinusOne> {
        &self.0.randomization()
    }

    pub fn data(&self) -> &[GenericArray<u8, A::BlockLength>] {
        self.0.data()
    }
}

#[derive(Clone)]
pub struct PublicKey<A>(State<A>)
where
    A: WOtsPlus;

impl<A> PublicKey<A>
where
    A: WOtsPlus,
{
    pub fn from_secret(secret_key: &SecretKey<A>) -> Self {
        match secret_key {
            &SecretKey(ref state) => PublicKey(state * Message::one()),
        }
    }

    pub fn randomization(
        &self,
    ) -> &GenericArray<GenericArray<u8, A::BlockLength>, A::WinternitzMinusOne> {
        &self.0.randomization()
    }

    pub fn data(&self) -> &[GenericArray<u8, A::BlockLength>] {
        self.0.data()
    }
}

#[derive(Clone)]
pub struct Signature<A>(State<A>)
where
    A: WOtsPlus;

impl<A> Signature<A>
where
    A: WOtsPlus,
    State<A>: Eq,
{
    pub fn sign(secret_key: &SecretKey<A>, message: GenericArray<u8, A::MessageSize>) -> Self {
        match secret_key {
            &SecretKey(ref state) => Signature(state * Message::message(message)),
        }
    }

    pub fn verify(
        &self,
        public_key: &PublicKey<A>,
        message: GenericArray<u8, A::MessageSize>,
    ) -> bool {
        let state = match self {
            &Signature(ref state) => state * Message::message(message).inverse(),
        };
        public_key.0 == state
    }

    pub fn randomization(
        &self,
    ) -> &GenericArray<GenericArray<u8, A::BlockLength>, A::WinternitzMinusOne> {
        &self.0.randomization()
    }

    pub fn data(&self) -> &[GenericArray<u8, A::BlockLength>] {
        self.0.data()
    }
}
