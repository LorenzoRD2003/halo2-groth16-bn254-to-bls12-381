//! This module contains utilities and traits for dealing with Fiat-Shamir
//! transcripts.
mod implementors;

use std::io::{self, Cursor, Read, Write};

/// Prefix to a prover's message soliciting a challenge
const BLAKE2B_PREFIX_CHALLENGE: u8 = 0;

/// Prefix to a prover's message
const BLAKE2B_PREFIX_COMMON: u8 = 1;

/// Hash function that can be used for transcript
pub trait TranscriptHash: Clone {
    /// Input type of the hash function
    type Input;
    /// Output type of the hash function
    type Output;

    /// Initialise the hasher
    fn init() -> Self;
    /// Absorb an element
    fn absorb(&mut self, input: &Self::Input);
    /// Squeeze an output
    fn squeeze(&mut self) -> Self::Output;
}

/// Traits to represent values that can be hashed with a `TranscriptHash`
pub trait Hashable<H: TranscriptHash>: Sized {
    /// Converts the hashable type to a format that can be hashed with `H`
    fn to_input(&self) -> H::Input;

    /// Converts the hashable type to bytes to be added to the transcript.
    fn to_bytes(&self) -> Vec<u8>;

    /// Reads bytes from a buffer and returns `Self`.
    fn read(buffer: &mut impl Read) -> io::Result<Self>;
}

/// Trait to represent values that can be sampled from a `TranscriptHash`
pub trait Sampleable<H: TranscriptHash> {
    /// Converts `H`'s output to Self
    fn sample(hash_output: H::Output) -> Self;
}

/// Generic transcript view
pub trait Transcript: Clone {
    /// Hash function
    type Hash: TranscriptHash;

    /// Initialises the transcript
    fn init() -> Self;

    /// Parses an existing transcript
    fn init_from_bytes(bytes: &[u8]) -> Self;

    /// Squeeze a challenge of type `T`, which only needs to be `Sampleable`
    /// with the corresponding hash function.
    fn squeeze_challenge<T: Sampleable<Self::Hash>>(&mut self) -> T;

    /// Writing a hashable element `T` to the transcript without writing it to
    /// the proof, treating it as a common commitment.
    fn common<T: Hashable<Self::Hash>>(&mut self, input: &T) -> io::Result<()>;

    /// Read a hashable element `T` from the prover.
    fn read<T: Hashable<Self::Hash>>(&mut self) -> io::Result<T>;

    /// Write a hashable element `T` to the proof and the transcript.
    fn write<T: Hashable<Self::Hash>>(&mut self, input: &T) -> io::Result<()>;

    /// Returns the buffer with the proof.
    fn finalize(self) -> Vec<u8>;

    /// Checks that the transcript is empty.
    /// This is used to make sure a transcript does not contain trailing bytes
    /// at the end of a proof verification.
    fn assert_empty(&mut self) -> io::Result<()>;
}

/// Transcript that can be snapshotted and restored for multi-stage proving.
pub trait PersistableTranscript: Transcript {
    /// Serializes the internal transcript state needed to continue proving
    /// later while preserving Fiat-Shamir challenge continuity.
    fn snapshot_state(&self) -> Vec<u8>;

    /// Restores a transcript from a serialized state snapshot plus the proof
    /// bytes already emitted before the continuation point.
    fn restore_from_state_and_bytes(state: &[u8], bytes: &[u8]) -> io::Result<Self>;
}

#[derive(Clone, Debug)]
/// Transcript used in proofs, parametrised by its hash function.
pub struct CircuitTranscript<H: TranscriptHash> {
    state: H,
    buffer: Cursor<Vec<u8>>,
}

impl<H: TranscriptHash> CircuitTranscript<H> {
    /// Returns the buffer for non default reading of the buffer (such as for
    /// reading an empty proof)
    pub fn buffer(&mut self) -> &mut Cursor<Vec<u8>> {
        &mut self.buffer
    }
}

impl<H: TranscriptHash> Transcript for CircuitTranscript<H> {
    type Hash = H;

    fn init() -> Self {
        Self {
            state: H::init(),
            buffer: Cursor::new(vec![]),
        }
    }

    fn init_from_bytes(bytes: &[u8]) -> Self {
        Self {
            state: H::init(),
            buffer: Cursor::new(bytes.to_vec()),
        }
    }

    fn squeeze_challenge<T: Sampleable<H>>(&mut self) -> T {
        T::sample(self.state.squeeze())
    }

    fn common<T: Hashable<H>>(&mut self, input: &T) -> io::Result<()> {
        self.state.absorb(&input.to_input());

        Ok(())
    }

    fn read<T: Hashable<H>>(&mut self) -> io::Result<T> {
        let val = T::read(&mut self.buffer)?;
        self.common(&val)?;

        Ok(val)
    }

    fn write<T: Hashable<H>>(&mut self, input: &T) -> io::Result<()> {
        self.common(input)?;
        let bytes = input.to_bytes();
        self.buffer.write_all(&bytes)
    }

    fn finalize(self) -> Vec<u8> {
        self.buffer.into_inner()
    }

    fn assert_empty(&mut self) -> io::Result<()> {
        if self.buffer.get_ref().len() == self.buffer.position() as usize {
            return Ok(());
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Transcript has unexpected trailing bytes.",
        ))
    }
}

#[derive(Clone, Debug)]
/// Transcript variant that records enough replay information to be restored
/// safely across split proving stages.
pub struct ReplayableCircuitTranscript<H: TranscriptHash<Input = Vec<u8>>> {
    state: H,
    buffer: Cursor<Vec<u8>>,
    replay_log: Vec<u8>,
}

impl<H: TranscriptHash<Input = Vec<u8>>> ReplayableCircuitTranscript<H> {
    fn log_common_input(&mut self, input: &[u8]) {
        self.replay_log.push(BLAKE2B_PREFIX_COMMON);
        self.replay_log
            .extend_from_slice(&(input.len() as u32).to_le_bytes());
        self.replay_log.extend_from_slice(input);
    }

    fn log_challenge_squeeze(&mut self) {
        self.replay_log.push(BLAKE2B_PREFIX_CHALLENGE);
    }
}

impl<H: TranscriptHash<Input = Vec<u8>>> Transcript for ReplayableCircuitTranscript<H> {
    type Hash = H;

    fn init() -> Self {
        Self {
            state: H::init(),
            buffer: Cursor::new(vec![]),
            replay_log: vec![],
        }
    }

    fn init_from_bytes(bytes: &[u8]) -> Self {
        Self {
            state: H::init(),
            buffer: Cursor::new(bytes.to_vec()),
            replay_log: vec![],
        }
    }

    fn squeeze_challenge<T: Sampleable<H>>(&mut self) -> T {
        self.log_challenge_squeeze();
        T::sample(self.state.squeeze())
    }

    fn common<T: Hashable<H>>(&mut self, input: &T) -> io::Result<()> {
        let input = input.to_input();
        self.log_common_input(&input);
        self.state.absorb(&input);

        Ok(())
    }

    fn read<T: Hashable<H>>(&mut self) -> io::Result<T> {
        let val = T::read(&mut self.buffer)?;
        self.common(&val)?;

        Ok(val)
    }

    fn write<T: Hashable<H>>(&mut self, input: &T) -> io::Result<()> {
        self.common(input)?;
        let bytes = input.to_bytes();
        self.buffer.write_all(&bytes)
    }

    fn finalize(self) -> Vec<u8> {
        self.buffer.into_inner()
    }

    fn assert_empty(&mut self) -> io::Result<()> {
        if self.buffer.get_ref().len() == self.buffer.position() as usize {
            return Ok(());
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Transcript has unexpected trailing bytes.",
        ))
    }
}

impl<H: TranscriptHash<Input = Vec<u8>>> PersistableTranscript for ReplayableCircuitTranscript<H> {
    fn snapshot_state(&self) -> Vec<u8> {
        self.replay_log.clone()
    }

    fn restore_from_state_and_bytes(state: &[u8], bytes: &[u8]) -> io::Result<Self> {
        let mut restored_state = H::init();
        let mut cursor = 0usize;
        while cursor < state.len() {
            let marker = state[cursor];
            cursor += 1;
            match marker {
                BLAKE2B_PREFIX_CHALLENGE => {
                    let _ = restored_state.squeeze();
                }
                BLAKE2B_PREFIX_COMMON => {
                    if cursor + 4 > state.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "truncated transcript replay log length prefix",
                        ));
                    }
                    let mut len_bytes = [0u8; 4];
                    len_bytes.copy_from_slice(&state[cursor..cursor + 4]);
                    cursor += 4;
                    let input_len = u32::from_le_bytes(len_bytes) as usize;
                    if cursor + input_len > state.len() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "truncated transcript replay log payload",
                        ));
                    }
                    restored_state.absorb(&state[cursor..cursor + input_len].to_vec());
                    cursor += input_len;
                }
                other => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unknown transcript replay marker: {other}"),
                    ));
                }
            }
        }

        let mut buffer = Cursor::new(bytes.to_vec());
        buffer.set_position(bytes.len() as u64);
        Ok(Self {
            state: restored_state,
            buffer,
            replay_log: state.to_vec(),
        })
    }
}

pub(crate) fn read_n<C, T>(transcript: &mut T, n: usize) -> io::Result<Vec<C>>
where
    T: Transcript,
    C: Hashable<T::Hash>,
{
    (0..n).map(|_| transcript.read()).collect()
}
