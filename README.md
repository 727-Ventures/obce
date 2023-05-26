# OpenBrush Chain Extension library

[![CI/CD](https://github.com/Brushfam/obce/actions/workflows/ci.yml/badge.svg)](https://github.com/Brushfam/obce/actions/workflows/ci.yml)

The library provides tools and primitives to simplify the development of chain 
extensions for ink! and Substrate.

OBCE automatically generates everything needed to correctly call chain extension
from ink! smart contracts, and to correctly implement the chain extension
itself on the Substrate side.

OBCE' macros automatically generate all the logic related to argument encoding/decoding,
function and extension identifier calculation and error handling.

The ink! side of OBCE is fully automated, while with Substrate all that's left is to
implement the chain extension using generated traits.

## Tutorial

For a step-by-step guide on how to create your own Substrate node with a custom chain extension
check the `rand-extension` example, which contains detailed instructions on the whole process.

## Usage examples

* `examples` directory
* [`pallet-assets`](https://github.com/727-Ventures/pallet-assets-chain-extension)