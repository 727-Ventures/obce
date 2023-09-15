use sp_weights::Weight;
use frame_system::Config as SysConfig;
use pallet_contracts::chain_extension::{
    BufInBufOutState,
    Environment,
    Ext,
    Result,
};
use sp_core::{
    Decode,
    MaxEncodedLen,
};
use sp_std::vec::Vec;

/// Generalized chain extension execution environment.
///
/// Custom [`ChainExtensionEnvironment`] implementations can be used for chain extension testing.
pub trait ChainExtensionEnvironment<E, T> {
    /// Opaque charged weight amount handle.
    type ChargedAmount;

    /// The function id within the `id` passed by a contract.
    ///
    /// It returns the two least significant bytes of the `id` passed by a contract as the other
    /// two bytes represent the chain extension itself (the code which is calling this function).
    fn func_id(&self) -> u16;

    /// The chain extension id within the `id` passed by a contract.
    ///
    /// It returns the two most significant bytes of the `id` passed by a contract which represent
    /// the chain extension itself (the code which is calling this function).
    fn ext_id(&self) -> u16;

    /// The length of the input as passed in as `input_len`.
    ///
    /// A chain extension would use this value to calculate the dynamic part of its
    /// weight. For example a chain extension that calculates the hash of some passed in
    /// bytes would use `in_len` to charge the costs of hashing that amount of bytes.
    /// This also subsumes the act of copying those bytes as a benchmarks measures both.
    fn in_len(&self) -> u32;

    /// Reads `min(max_len, in_len)` from contract memory.
    ///
    /// This does **not** charge any weight. The caller must make sure that the an
    /// appropriate amount of weight is charged **before** reading from contract memory.
    /// The reason for that is that usually the costs for reading data and processing
    /// said data cannot be separated in a benchmark. Therefore a chain extension would
    /// charge the overall costs either using `max_len` (worst case approximation) or using
    /// [`in_len()`](Self::in_len).
    fn read(&self, max_len: u32) -> Result<Vec<u8>>;

    /// Reads `min(buffer.len(), in_len) from contract memory.
    ///
    /// This takes a mutable pointer to a buffer fills it with data and shrinks it to
    /// the size of the actual data. Apart from supporting pre-allocated buffers it is
    /// equivalent to to [`read()`](Self::read).
    fn read_into(&self, buffer: &mut &mut [u8]) -> Result<()>;

    /// Reads and decodes a type with a size fixed at compile time from contract memory.
    ///
    /// This function is secure and recommended for all input types of fixed size
    /// as long as the cost of reading the memory is included in the overall already charged
    /// weight of the chain extension. This should usually be the case when fixed input types
    /// are used.
    fn read_as<U: Decode + MaxEncodedLen>(&mut self) -> Result<U>;

    /// Reads and decodes a type with a dynamic size from contract memory.
    ///
    /// Make sure to include `len` in your weight calculations.
    fn read_as_unbounded<U: Decode>(&mut self, len: u32) -> Result<U>;

    /// Write the supplied buffer to contract memory.
    ///
    /// If the contract supplied buffer is smaller than the passed `buffer` an `Err` is returned.
    /// If `allow_skip` is set to true the contract is allowed to skip the copying of the buffer
    /// by supplying the guard value of `pallet-contracts::SENTINEL` as `out_ptr`. The
    /// `weight_per_byte` is only charged when the write actually happens and is not skipped or
    /// failed due to a too small output buffer.
    fn write(&mut self, buffer: &[u8], allow_skip: bool, weight_per_byte: Option<Weight>) -> Result<()>;

    /// Charge the passed `amount` of weight from the overall limit.
    ///
    /// It returns `Ok` when there the remaining weight budget is larger than the passed
    /// `weight`. It returns `Err` otherwise. In this case the chain extension should
    /// abort the execution and pass through the error.
    ///
    /// # Note
    ///
    /// Weight is synonymous with gas in substrate.
    fn charge_weight(&mut self, amount: Weight) -> Result<Self::ChargedAmount>;

    /// Adjust a previously charged amount down to its actual amount.
    ///
    /// This is when a maximum a priori amount was charged and then should be partially
    /// refunded to match the actual amount.
    fn adjust_weight(&mut self, charged: Self::ChargedAmount, actual_weight: Weight);

    /// Grants access to the execution environment of the current contract call.
    ///
    /// Consult the functions on the returned type before re-implementing those functions.
    fn ext(&mut self) -> &mut E;
}

impl<'a, 'b, E, T> ChainExtensionEnvironment<E, T> for Environment<'a, 'b, E, BufInBufOutState>
where
    T: SysConfig,
    E: Ext<T = T>,
{
    type ChargedAmount = pallet_contracts::chain_extension::ChargedAmount;

    fn func_id(&self) -> u16 {
        Environment::func_id(self)
    }

    fn ext_id(&self) -> u16 {
        Environment::ext_id(self)
    }

    fn in_len(&self) -> u32 {
        Environment::in_len(self)
    }

    fn read(&self, max_len: u32) -> Result<Vec<u8>> {
        Environment::read(self, max_len)
    }

    fn read_into(&self, buffer: &mut &mut [u8]) -> Result<()> {
        Environment::read_into(self, buffer)
    }

    fn read_as<U: Decode + MaxEncodedLen>(&mut self) -> Result<U> {
        Environment::read_as(self)
    }

    fn read_as_unbounded<U: Decode>(&mut self, len: u32) -> Result<U> {
        Environment::read_as_unbounded(self, len)
    }

    fn write(&mut self, buffer: &[u8], allow_skip: bool, weight_per_byte: Option<Weight>) -> Result<()> {
        Environment::write(self, buffer, allow_skip, weight_per_byte)
    }

    fn charge_weight(&mut self, amount: Weight) -> Result<Self::ChargedAmount> {
        Environment::charge_weight(self, amount)
    }

    fn adjust_weight(&mut self, charged: Self::ChargedAmount, actual_weight: Weight) {
        Environment::adjust_weight(self, charged, actual_weight)
    }

    fn ext(&mut self) -> &mut E {
        Environment::ext(self)
    }
}
