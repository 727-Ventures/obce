# OBCE usage example

This example is a port of a [rand-extension example](https://github.com/paritytech/ink/tree/master/integration-tests/rand-extension) from ink! repo.

As in the original example, this one provides you with:

* ink! smart contract, that calls the chain extension
* Substrate extension

## Details

File structure of the chain extension is as follows:

* `lib.rs` - defines the glue code between Substrate and ink! parts of the chain extension.
* `ink.rs` - contains a struct, which can be used by ink! smart contracts to interact with your chain extension.
* `substrate.rs` - contains the implementation of the chain extension for Substrate-based chains.

### `lib.rs`

Using `#[obce::definition]` macro, `lib.rs` provides us with the automatically generated
code that ensures chain extension identifier, method identifier and method ABI matching between ink! and Substrate.

To provide a stable chain extension identifier for registry purposes, we specify an optional `id` parameter
when using `#[obce::definition]` macro.

We also have a custom error enumeration defined with `#[obce::error]` macro.

### `ink.rs`

The ink! part of the chain extension is fairly simple, since most of the work is already done
by `#[obce::definition]` macro we used previously.

When used with `ink` feature enabled, `#[obce::definition]` macro generated glue code methods
as default trait methods, so to start using them we need to implement this trait for some struct.

This struct is also marked with the `#[obce::ink_lang::extension]` macro, which automatically
generates the necessary `impl` blocks to make sure that you can use your chain extension
in the [ink! environment](https://use.ink/basics/environment-functions) context.

### `substrate.rs`

The Substrate part of the chain extension requires a single trait `impl` block
marked with `#[obce::implementation]` macro.

For more information on generics and their trait bounds check documentation
for `#[obce::implementation]` macro. In general, the bounds are similar to what
you would use by implementing chain extension from scratch, but with the introduction
of a more easily testable `ChainExtensionEnvironment` trait.

## Example integration

### Substrate

In this section, we will initialize a `substrate-contracts-node` with our own chain extension,
which can later be used by the smart contracts that are deployed on this node.

1. Clone the [`substrate-contracts-node`](https://github.com/paritytech/substrate-contracts-node) repository.
2. Copy the `chain-extension` crate into your project and add it as a workspace member to `Cargo.toml` file:

```toml
# ...

members = [
    'chain-extension',
    'node',
    'runtime'
]

# ...
```

3. Add `rand-extension` to `runtime/Cargo.toml` file:

```toml
rand-extension = { path = "../chain-extension", default-features = false, features = ["substrate"] }
```

Also, add `rand-extension/substrate-std` to feature list that is activated when `std` feature is enabled:

```toml
[features]
# ...

std = [
    # ...
    "rand-extension/substrate-std",
]
```

4. Change `pallet_contracts::Config` to use the chain extension like so:

```rust
impl pallet_contracts::Config for Runtime {
    // ...

    type ChainExtension = (
        pallet_assets_chain_extension::substrate::AssetsExtension,

        // Your custom extension
        rand_extension::substrate::Extension,
    );

    // ...
}
```

5. Launch your node with the `cargo run` command.

### ink!

1. Make sure that you have [`cargo-contract`](https://github.com/paritytech/cargo-contract#installation) installed.
2. Create new ink! contract with `cargo contract new` command.
3. Replace lib.rs and Cargo.toml files with the example ones.
4. Modify path to your chain extension crate in the `Cargo.toml` file.
5. Build your contract with `cargo contract build` command.

You can utilize [Contracts UI](https://contracts-ui.substrate.io/) interface
to test your contract on the previously built node.
