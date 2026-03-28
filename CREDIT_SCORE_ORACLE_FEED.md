# SoroSusu Credit Score Oracle Feed

## Overview

The most valuable byproduct of participating in a SoroSusu circle is the **Proof of Reliability**. 

By building a read-only Oracle feed that returns a user's "Reliability Score", SoroSusu provides a **Reputation-as-a-Service** primitive to the broader Stellar and Soroban ecosystem. This acts as the foundation for undercollateralized lending, allowing users to leverage their on-chain savings history as collateral for business loans, leasing applications, or reduced protocol fees.

---

## The Scoring Algorithm

The oracle returns an integer out of a maximum **1000 points**:

1. **Reliability (Up to 700 pts)**: Weighted heavily on the ratio of on-time contributions to total contributions. Even a single late payment or insurance bailout will significantly drag this metric down.
2. **Experience (Up to 200 pts)**: Rewards longevity and frequent use of the protocol. Users gain 20 points per successful on-time contribution up to the cap.
3. **Volume (Up to 100 pts)**: Scales logarithmically based on the total financial volume a user has reliably saved/managed through SoroSusu.

---

## On-Chain Interoperability (Rust)

Other Soroban smart contracts (e.g., a Lending Pool or a Vesting Vault) can query a user's credit score directly on-chain to determine risk parameters.

```rust
use soroban_sdk::{contract, contractimpl, Address, Env};

// 1. Define the Oracle Interface
#[contractclient(name = "SoroSusuClient")]
pub trait SoroSusuTrait {
    fn get_user_reliability_score(env: Env, user: Address) -> u32;
}

// 2. Implement in your consumer contract
#[contract]
pub struct UndercollateralizedLending;

#[contractimpl]
impl UndercollateralizedLending {
    pub fn request_loan(env: Env, oracle_address: Address, borrower: Address) {
        let client = SoroSusuClient::new(&env, &oracle_address);
        
        // Fetch the user's reputation score from SoroSusu
        let score = client.get_user_reliability_score(&borrower);
        
        if score < 650 {
            panic!("Credit score too low for undercollateralized borrowing.");
        }
        
        // Proceed with loan issuance...
    }
}
```

---

## Off-Chain Interoperability (TypeScript)

If you are building a decentralized frontend (DApp) that wants to display a user's SoroSusu score, you can utilize the `@stellar/stellar-sdk` to simulate a transaction against the read-only function.

```typescript
import { rpc, Contract, nativeToScVal, scValToNative } from '@stellar/stellar-sdk';

const SOROSUSU_CONTRACT_ID = 'C...'; // Replace with deployed SoroSusu ID
const rpcServer = new rpc.Server('https://soroban-testnet.stellar.org');
const contract = new Contract(SOROSUSU_CONTRACT_ID);

export async function checkUserCreditScore(userAddress: string): Promise<number> {
    try {
        // Prepare the contract call
        const tx = contract.call(
            'get_user_reliability_score', 
            nativeToScVal(userAddress, { type: 'address' })
        );
        
        // Since it's a read-only data query, we only need to simulate it
        const simResult = await rpcServer.simulateTransaction(tx);
        
        if (rpc.Api.isSimulationSuccess(simResult)) {
            // Parse the returned u32 value
            const score = scValToNative(simResult.result.retval);
            console.log(`Address ${userAddress} has a score of: ${score}`);
            return Number(score);
        } else {
            throw new Error('Simulation failed or user has no data.');
        }
    } catch (error) {
        console.error("Error querying SoroSusu Oracle:", error);
        return 0;
    }
}
```

---

## Sybil Resistance Strategies

A major concern with reputation systems is "Sybil attacks," where a user creates two wallets and passes funds back and forth to artificially inflate their score. SoroSusu mitigates this organically through:

1. **Time-Locked Capital:** Building experience points requires holding funds in the contract throughout the duration of a cycle. Passing funds rapidly back and forth is impossible due to the locked deadlines of ROSCA cycles.
2. **Protocol Fees:** Running dummy circles to fake volume incurs the base protocol fee, making a volume-based Sybil attack an expensive endeavor.
3. **Minimum Member Requirements:** To prevent 2-wallet echo chambers, production circles can enforce a minimum global requirement of 4+ participants.

*Note: For strict institutional lending parameters, SoroSusu's Oracle feed should be combined with a "Proof of Humanity" system or minimal KYC verifier on the consumer side.*