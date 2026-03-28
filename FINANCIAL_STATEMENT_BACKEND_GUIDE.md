# Source of Funds Verification - Backend Integration Guide

## Overview

The FinancialStatement contract helper provides a robust system for generating cryptographically signed financial statements that users can present to banks as proof of "Source of Funds" for Susu payouts. This protects users from unnecessary tax flags or account freezes when receiving large payouts (e.g., $5,000+).

## Architecture

### 1. Smart Contract Components

- **FinancialTransaction**: Tracks all contributions, payouts, penalties, and insurance fees
- **FinancialStatement**: Contains hashed summary of all transactions within a period
- **StatementMetadata**: Circle information for PDF generation
- **Keccak256 Hash**: Cryptographic proof of data integrity

### 2. Backend Integration Points

The contract provides several functions for backend integration:

#### `generate_financial_statement()`
Generates a signed financial statement for a specific member and time period.

**Parameters:**
- `requesting_member`: Address of the member requesting the statement
- `circle_id`: ID of the Susu circle
- `period_start`: Unix timestamp for period start
- `period_end`: Unix timestamp for period end

**Returns:**
- `FinancialStatement` struct with all totals and cryptographic hash

#### `get_pdf_generation_data()`
Convenience function that returns all data needed for PDF generation in one call.

**Returns:**
- `FinancialStatement`: Summary data and hash
- `Vec<TransactionRecord>`: Detailed transaction list
- `StatementMetadata`: Circle information

#### `verify_statement_hash()`
Allows backend to verify statement authenticity without trusting user-provided data.

## PDF Generation Implementation

### Backend Service Example (Node.js)

```javascript
const { SorobanRpc } = require('@soroban/rpc');
const { Contract } = require('@soroban/contract');

class FinancialStatementPDFGenerator {
  constructor(rpcUrl, contractId) {
    this.rpc = new SorobanRpc(rpcUrl);
    this.contract = new Contract(contractId);
  }

  async generatePDF(memberAddress, circleId, periodStart, periodEnd) {
    try {
      // 1. Get financial statement data from contract
      const statementData = await this.getStatementData(
        memberAddress, 
        circleId, 
        periodStart, 
        periodEnd
      );

      // 2. Verify hash integrity
      const isValid = await this.verifyStatementHash(
        circleId,
        statementData.statement.statement_hash,
        periodStart,
        periodEnd
      );

      if (!isValid) {
        throw new Error('Statement hash verification failed');
      }

      // 3. Generate PDF
      const pdfBuffer = await this.createPDF(statementData);

      // 4. Store verification metadata
      await this.storeVerificationMetadata({
        memberAddress,
        circleId,
        periodStart,
        periodEnd,
        statementHash: statementData.statement.statement_hash,
        generatedAt: new Date().toISOString(),
        verified: true
      });

      return pdfBuffer;

    } catch (error) {
      console.error('PDF generation failed:', error);
      throw error;
    }
  }

  async getStatementData(memberAddress, circleId, periodStart, periodEnd) {
    const result = await this.rpc.invokeContract({
      contract: this.contract.address(),
      method: 'get_pdf_generation_data',
      args: [
        new Address(memberAddress),
        new NativeInt64(circleId),
        new NativeInt64(periodStart),
        new NativeInt64(periodEnd)
      ]
    });

    return this.parseStatementData(result);
  }

  async createPDF(statementData) {
    const { statement, transactions, metadata } = statementData;
    
    // Use PDF generation library (e.g., PDFKit, Puppeteer)
    const PDFDocument = require('pdfkit');
    const doc = new PDFDocument();

    // Header
    doc.fontSize(20).text('Susu Group - Source of Funds Verification', { align: 'center' });
    doc.moveDown();

    // Circle Information
    doc.fontSize(14).text('Circle Information:');
    doc.fontSize(12).text(`Circle ID: ${metadata.circle_id}`);
    doc.text(`Creator: ${metadata.circle_creator}`);
    doc.text(`Contribution Amount: ${this.formatAmount(metadata.contribution_amount)}`);
    doc.text(`Max Members: ${metadata.max_members}`);
    doc.text(`Current Round: ${metadata.current_round}`);
    doc.moveDown();

    // Statement Summary
    doc.fontSize(14).text('Financial Summary:');
    doc.fontSize(12).text(`Statement Period: ${new Date(statement.statement_period_start * 1000).toLocaleDateString()} - ${new Date(statement.statement_period_end * 1000).toLocaleDateString()}`);
    doc.text(`Total Contributions: ${this.formatAmount(statement.total_contributions)}`);
    doc.text(`Total Payouts: ${this.formatAmount(statement.total_payouts)}`);
    doc.text(`Total Penalties: ${this.formatAmount(statement.total_penalties)}`);
    doc.text(`Total Insurance Fees: ${this.formatAmount(statement.total_insurance_fees)}`);
    doc.text(`Net Amount: ${this.formatAmount(statement.net_amount)}`);
    doc.text(`Transaction Count: ${statement.transaction_count}`);
    doc.moveDown();

    // Verification Section
    doc.fontSize(14).text('Verification Information:');
    doc.fontSize(12).text(`Statement Hash: ${statement.statement_hash.toString('hex')}`);
    doc.text(`Generated At: ${new Date(statement.generated_at * 1000).toISOString()}`);
    doc.text(`Verifying Member: ${statement.verifying_member}`);
    doc.moveDown();

    // Transaction Details Table
    doc.fontSize(14).text('Transaction Details:');
    this.createTransactionTable(doc, transactions);

    // Footer with legal disclaimer
    doc.fontSize(10).text('This document serves as proof that the funds received are from a communal savings group (ROSCA) and not taxable income.', { align: 'center' });
    doc.text('All transactions are verifiable on the Stellar blockchain.', { align: 'center' });

    return doc;
  }

  createTransactionTable(doc, transactions) {
    const tableTop = doc.y;
    const itemHeight = 20;
    const tableHeaders = ['Date', 'Type', 'Amount', 'Round', 'Status'];
    
    // Table headers
    let xPos = 50;
    tableHeaders.forEach(header => {
      doc.text(header, xPos, tableTop, { width: 100 });
      xPos += 100;
    });

    // Table rows
    transactions.forEach((tx, index) => {
      const yPos = tableTop + (index + 1) * itemHeight;
      let xPos = 50;
      
      doc.text(new Date(tx.timestamp * 1000).toLocaleDateString(), xPos, yPos, { width: 100 });
      xPos += 100;
      doc.text(tx.transaction_type, xPos, yPos, { width: 100 });
      xPos += 100;
      doc.text(this.formatAmount(tx.amount), xPos, yPos, { width: 100 });
      xPos += 100;
      doc.text(tx.round_number.toString(), xPos, yPos, { width: 100 });
      xPos += 100;
      doc.text(tx.is_late ? 'Late' : 'On Time', xPos, yPos, { width: 100 });
    });
  }

  formatAmount(amount) {
    // Assuming 7 decimal places (Stellar standard)
    return (amount / 10000000).toLocaleString('en-US', {
      style: 'currency',
      currency: 'USD'
    });
  }

  parseStatementData(result) {
    // Parse the Soroban contract response into structured data
    return {
      statement: {
        circle_id: result[0].circle_id,
        statement_period_start: result[0].statement_period_start,
        statement_period_end: result[0].statement_period_end,
        total_contributions: result[0].total_contributions,
        total_payouts: result[0].total_payouts,
        total_penalties: result[0].total_penalties,
        total_insurance_fees: result[0].total_insurance_fees,
        net_amount: result[0].net_amount,
        transaction_count: result[0].transaction_count,
        statement_hash: result[0].statement_hash,
        generated_at: result[0].generated_at,
        verifying_member: result[0].verifying_member
      },
      transactions: result[1],
      metadata: result[2]
    };
  }
}
```

## API Endpoint Example

```javascript
// Express.js endpoint
app.post('/api/financial-statement/pdf', async (req, res) => {
  try {
    const { memberAddress, circleId, periodStart, periodEnd } = req.body;
    
    // Validate input
    if (!memberAddress || !circleId || !periodStart || !periodEnd) {
      return res.status(400).json({ error: 'Missing required parameters' });
    }

    // Generate PDF
    const pdfGenerator = new FinancialStatementPDFGenerator(
      process.env.SOROBAN_RPC_URL,
      process.env.FINANCIAL_STATEMENT_CONTRACT_ID
    );

    const pdfBuffer = await pdfGenerator.generatePDF(
      memberAddress,
      circleId,
      periodStart,
      periodEnd
    );

    // Set appropriate headers
    res.setHeader('Content-Type', 'application/pdf');
    res.setHeader('Content-Disposition', `attachment; filename="susu-statement-${circleId}-${periodStart}.pdf"`);
    res.send(pdfBuffer);

  } catch (error) {
    console.error('PDF generation error:', error);
    res.status(500).json({ error: 'Failed to generate financial statement PDF' });
  }
});
```

## Security Considerations

### 1. Hash Verification
Always verify the statement hash on-chain before generating PDFs. This prevents tampering.

### 2. Rate Limiting
Implement rate limiting on PDF generation endpoints to prevent abuse.

### 3. Access Control
Ensure only authorized members can request their own financial statements.

### 4. Data Privacy
Consider encrypting PDFs at rest and implementing secure delivery mechanisms.

## Bank Integration Tips

### 1. Standard Format
The PDF should include:
- Clear "Source of Funds" designation
- Transaction history with dates and amounts
- Cryptographic verification hash
- Circle information and member count
- Legal disclaimer about communal savings

### 2. Verification Process
Provide banks with:
- Instructions to verify the hash on Stellar Explorer
- Contract address for independent verification
- Explanation of ROSCA (Rotating Savings and Credit Association) structure

### 3. Compliance Features
- Include member's contribution history
- Show consistent payment patterns
- Demonstrate group participation over time

## Testing

Use the provided test functions to verify:
- Hash generation consistency
- Transaction tracking accuracy
- PDF generation workflow
- Backend integration points

## Deployment

1. Deploy the FinancialStatement contract
2. Configure backend service with contract address
3. Set up RPC endpoints
4. Implement PDF generation service
5. Add monitoring and logging
6. Test with real transaction data

This implementation provides a complete solution for Source of Funds verification that protects users while maintaining regulatory compliance.
