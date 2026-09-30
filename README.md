# Solana program

- Program id: `NRXxKGB1hLEfL2Kc2kCGQW4NFNvh7cpLx7eKgdzeLXt` (Solana mainnet)

## Verifiable build

```bash
solana-verify build --library-name commerce_town
solana-verify get-executable-hash target/deploy/commerce_town.so
solana-verify get-program-hash -um NRXxKGB1hLEfL2Kc2kCGQW4NFNvh7cpLx7eKgdzeLXt
```

The two hashes must be equal. The same build runs in this repository's "Verifiable build" workflow.
