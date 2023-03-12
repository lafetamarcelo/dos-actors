# Clients

The crate `gmt_dos-clients` includes a library of clients for signals generation and signal processing.

To use some of the clients in `gmt_dos-clients`, add the crate to your list of dependencies with 
```
cargo add gmt_dos-clients
```
If you are only looking for the `gmt_dos-actors` interface, you can instead do
```
cargo add gmt_dos-clients --no-default-features --features interface
```