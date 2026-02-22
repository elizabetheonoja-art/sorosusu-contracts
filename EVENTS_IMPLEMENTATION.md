# SoroSusu Events Implementation

## Issue #25: Events - Emit CycleCompleted and GroupRollover

**Acceptance Criteria Met:**
- ✅ Emit `CycleCompleted(group_id, total_volume_distributed)` when the last member gets paid
- ✅ Emit `GroupRollover(group_id, new_cycle_number)` when the admin restarts the group

## Event Structures

### CycleCompletedEvent
```rust
pub struct CycleCompletedEvent {
    group_id: u32,
    total_volume_distributed: i128,
}
```
- **Trigger**: Emitted when the last member of a cycle receives their payout
- **Event Symbol**: `CYCLE_COMP`
- **Use Case**: Backend indexer updates historical leaderboards and UI states

### GroupRolloverEvent
```rust
pub struct GroupRolloverEvent {
    group_id: u32,
    new_cycle_number: u32,
}
```
- **Trigger**: Emitted when admin calls `rollover_group()` to start a new cycle
- **Event Symbol**: `GROUP_ROLL`
- **Use Case**: Backend indexer tracks cycle progression and updates analytics

## Enhanced Circle Structure

```rust
pub struct Circle {
    admin: Address,
    contribution: i128,
    members: Vec<Address>,
    cycle_number: u32,                // NEW: Current cycle tracking
    current_payout_index: u32,        // NEW: Payout progress tracking
    has_received_payout: Vec<bool>,   // NEW: Per-member payout status
    total_volume_distributed: i128,    // NEW: Total volume for current cycle
}
```

## Key Functions

### Core Operations
- `process_payout(env, circle_id, recipient)` - Processes individual payouts and emits `CycleCompleted` when cycle finishes
- `rollover_group(env, circle_id)` - Resets group for next cycle and emits `GroupRollover` event

### Query Functions
- `get_cycle_info(env, circle_id)` - Returns `(cycle_number, current_payout_index, total_volume_distributed)`
- `get_payout_status(env, circle_id)` - Returns payout status vector for all members

## Event Emission Logic

### CycleCompleted Event
```rust
if all_paid {
    let event = CycleCompletedEvent {
        group_id: circle_id,
        total_volume_distributed: circle.total_volume_distributed,
    };
    event::publish(&env, symbol_short!("CYCLE_COMP"), &event);
}
```

### GroupRollover Event
```rust
let event = GroupRolloverEvent {
    group_id: circle_id,
    new_cycle_number: circle.cycle_number,
};
event::publish(&env, symbol_short!("GROUP_ROLL"), &event);
```

## Backend Integration

### Event Monitoring
Backend systems can monitor these events to:
- Update historical leaderboards when cycles complete
- Track group performance metrics
- Maintain real-time UI state synchronization
- Calculate analytics on volume distribution

### Event Data Structure
Events are published with:
- **Event Symbol**: Short symbol for easy identification
- **Event Data**: Structured data with relevant parameters
- **Timestamp**: Automatically included by Soroban

## Security Features

- **Admin-only operations**: Only circle admin can process payouts and rollover groups
- **Duplicate prevention**: Members cannot receive payout twice in same cycle
- **Cycle completion validation**: Rollover only allowed after all members paid
- **Member verification**: Payouts only go to verified circle members

## Testing Coverage

Comprehensive tests verify:
- ✅ Event emission on cycle completion
- ✅ Event emission on group rollover
- ✅ Event data accuracy
- ✅ Authorization controls
- ✅ Error conditions
- ✅ Volume tracking accuracy

## Usage Flow

1. **Create Circle**: Admin creates circle with contribution amount
2. **Members Join**: Members join the circle
3. **Process Payouts**: Admin calls `process_payout()` for each member
4. **Cycle Completion**: Last payout triggers `CycleCompleted` event
5. **Group Rollover**: Admin calls `rollover_group()` → emits `GroupRollover` event
6. **Next Cycle**: Process repeats with incremented cycle number

## Event Examples

### CycleCompleted Event
```rust
// When 3-member circle with 100 USDC contribution completes
CycleCompletedEvent {
    group_id: 123,
    total_volume_distributed: 300_i128,  // 3 members × 100 USDC
}
```

### GroupRollover Event
```rust
// When circle moves from cycle 1 to cycle 2
GroupRolloverEvent {
    group_id: 123,
    new_cycle_number: 2,
}
```

This implementation provides the backend indexer with all necessary information to maintain accurate historical records and real-time UI updates.
