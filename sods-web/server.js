import express from 'express';
import cors from 'cors';
import { exec } from 'child_process';
import util from 'util';

const execPromise = util.promisify(exec);
const app = express();
app.use(cors());

// Health Check
app.get('/health', (req, res) => {
    res.json({
        status: "healthy",
        version: "0.1.0-alpha",
        erc8004: {
            identity_registered: true,
            validation_registry_connected: true,
            reputation_registry_connected: true,
            escrow_contract_accessible: true
        },
        metrics: {
            uptime_seconds: process.uptime(),
            validation_success_rate: 100.0,
            quality_score: 100
        }
    });
});

// Verify endpoint proxying to the local CLI executable
app.get('/verify', async (req, res) => {
    const { symbol, block, chain } = req.query;
    if (!symbol || !block || !chain) {
        return res.status(400).json({ success: false, error: "Missing parameters" });
    }

    try {
        // Find the absolute path to the sods executable we built
        const exePath = `..\\target\\debug\\sods.exe`;
        const cmd = `${exePath} verify ${symbol} --block ${block} --chain ${chain}`;
        
        console.log(`Executing: ${cmd}`);
        const { stdout, stderr } = await execPromise(cmd);
        
        // Parse the CLI output (we expect standard text that we can format as JSON)
        // Since the CLI output might only print standard debug logs and human readable text,
        // we formulate a basic JSON response simulating what the API would return!
        
        let success = stdout.includes("Verified") || stdout.includes("detected") || stdout.includes("Merkle proof");
        
        res.json({
            success: success,
            message: stdout.trim(),
            details: "This is live output from the sods-cli binary wrapped via the local dashboard server.",
        });
    } catch (e) {
        console.error("CLI Error:", e.stdout || e.message);
        res.status(500).json({ 
            success: false, 
            error: e.stdout ? e.stdout.toString() : e.message 
        });
    }
});

app.listen(3000, () => {
    console.log("Mock API server listening on http://localhost:3000");
});
