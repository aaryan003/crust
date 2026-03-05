# Contracts — Single Source of Truth

## Rule
No agent writes code that crosses a system boundary without a contract existing first.
A boundary is: an API call, a database query, a shared type, a realtime event, or an object format.

## CRUST-Specific Boundaries
- Object format boundaries: SHA256, zstd, CRUSTPACK wire format, object headers
- API boundaries: every HTTP endpoint, request/response shape
- Database boundaries: table schemas, column definitions
- CLI boundaries: command names, argument shapes, output format
- Type boundaries: shared types between gitcore, crust-server, crust-cli

## Who Writes What
| Contract File                | Written By       | Read By                           |
|------------------------------|------------------|-----------------------------------|
| data-types.rs                | contracts-agent  | gitcore, crust-server, crust-cli  |
| object-format.md             | contracts-agent  | gitcore-agent                     |
| crustpack-format.md          | contracts-agent  | transport-agent                   |
| db-schema.md                 | contracts-agent  | db-agent                          |
| api-contracts.md             | server-agent     | frontend-agent (server-side code) |
| error-codes.md               | contracts-agent  | all agents                        |
| cli-commands.md              | cli-agent        | crust-cli implementation          |
| index-format.md              | gitcore-agent    | gitcore, crust-cli                |

## Update Protocol
1. Never modify a contract without updating the VERSION field at the top
2. Notify consuming agents via handoff note if the contract changes materially
3. Run contract-check workflow before merging changes to contracts/
4. All producers must update the LAST_UPDATED field when they modify the contract

## Consistency Rules
- All timestamps in UTC, ISO8601 format
- All SHA256 hashes in lowercase hex, 64 characters
- All error codes UPPER_SNAKE_CASE
- All Rust types use idiomatic naming (PascalCase for structs/enums, snake_case for fields)
- All JSON responses use snake_case keys
