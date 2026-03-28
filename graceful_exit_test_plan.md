# Graceful Exit Implementation Test Plan

## Test Cases for request_exit() function:

1. **Happy Path**: Active member requests exit
   - Member status changes from Active to AwaitingReplacement
   - PendingExit record is created
   - Member's position in queue is locked

2. **Error Cases**:
   - Non-member tries to request exit → Should panic
   - Member in AwaitingReplacement state tries to request exit → Should panic
   - Ejected member tries to request exit → Should panic

## Test Cases for fill_vacancy() function:

1. **Happy Path**: New member fills vacancy
   - Exiting member receives refund of total_contributed
   - Exiting member status changes to Ejected
   - New member inherits the same index position
   - New member gets Active status
   - NFT is burned from exiting member and minted to new member
   - PendingExit record is removed

2. **Error Cases**:
   - No pending exit exists for specified member → Should panic
   - Exiting member not in AwaitingReplacement state → Should panic
   - New member already in a circle → Should panic

## Integration Tests:

1. **Full Graceful Exit Flow**:
   - Member joins circle and makes contributions
   - Member requests graceful exit
   - New member fills vacancy
   - Verify exiting member gets correct refund
   - Verify queue position is preserved

2. **Queue Position Preservation**:
   - Create circle with multiple members
   - Member at position 2 requests exit
   - New member fills vacancy
   - Verify new member takes position 2 in payout queue

## Data Structure Validation:

✅ MemberStatus enum with Active, AwaitingReplacement, Ejected states
✅ Member struct with status and total_contributed fields  
✅ PendingExit storage key to track exit requests
✅ Trait methods added to SoroSusuTrait

## Logic Validation:

✅ Pro-rata settlement: Only principal contributions refunded
✅ Queue position inheritance: New member gets same index
✅ NFT transfer: Burn from exiting, mint to new member
✅ State management: Proper status transitions

## Security Considerations:

✅ Authorization checks with require_auth()
✅ State validation to prevent double exits
✅ Proper cleanup of storage records
✅ Protection against unauthorized vacancy filling

The implementation follows the acceptance criteria:
- ✅ Implement request_exit()
- ✅ Change member state to AwaitingReplacement  
- ✅ Lock their turn in the queue until fill_vacancy() is called
- ✅ Pro-rata settlement (principal only refund after replacement found)
