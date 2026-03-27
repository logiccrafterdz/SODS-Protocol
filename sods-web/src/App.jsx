import { useState, useEffect, useCallback } from 'react'
import './App.css'

const SYMBOLS = [
  { code: 'Tf', name: 'ERC20 Transfer' },
  { code: 'Dep', name: 'WETH Deposit' },
  { code: 'Wdw', name: 'WETH Withdrawal' },
  { code: 'Sw', name: 'Uniswap V2/V3 Swap' },
  { code: 'LP+', name: 'Add Liquidity (Mint)' },
  { code: 'LP-', name: 'Remove Liquidity (Burn)' },
  { code: 'MintNFT', name: 'ERC721/1155 Mint' },
  { code: 'BuyNFT', name: 'NFT Purchase (Seaport)' },
  { code: 'BridgeIn', name: 'L1→L2 Bridge Deposit' },
  { code: 'BridgeOut', name: 'L2→L1 Withdrawal' },
  { code: 'Sandwich', name: 'MEV Sandwich Pattern' },
  { code: 'AAOp', name: 'ERC-4337 UserOp' },
  { code: 'Permit2', name: 'Gasless Approval' },
  { code: 'CoWTrade', name: 'CoW Swap Intent' },
]

const API_BASE = 'http://localhost:3000'

function App() {
  const [health, setHealth] = useState(null)
  const [symbol, setSymbol] = useState('Tf')
  const [block, setBlock] = useState('')
  const [chain, setChain] = useState('sepolia')
  const [verifyResult, setVerifyResult] = useState(null)
  const [loading, setLoading] = useState(false)
  const [uptime, setUptime] = useState(0)

  // Simulated uptime counter
  useEffect(() => {
    const interval = setInterval(() => setUptime(prev => prev + 1), 1000)
    return () => clearInterval(interval)
  }, [])

  // Health check
  const checkHealth = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/health`, { signal: AbortSignal.timeout(5000) })
      if (res.ok) {
        const data = await res.json()
        setHealth({ status: 'connected', ...data })
      } else {
        setHealth({ status: 'degraded' })
      }
    } catch {
      setHealth({ status: 'offline' })
    }
  }, [])

  useEffect(() => {
    checkHealth()
    const interval = setInterval(checkHealth, 15000)
    return () => clearInterval(interval)
  }, [checkHealth])

  // Verify symbol
  const handleVerify = async (e) => {
    e.preventDefault()
    if (!block) return
    setLoading(true)
    setVerifyResult(null)

    try {
      const res = await fetch(
        `${API_BASE}/verify?symbol=${encodeURIComponent(symbol)}&block=${block}&chain=${chain}`,
        { signal: AbortSignal.timeout(30000) }
      )
      const data = await res.json()
      setVerifyResult(data)
    } catch (err) {
      setVerifyResult({
        success: false,
        error: `Connection failed: ${err.message}. Make sure the SODS daemon is running on port 3000.`
      })
    } finally {
      setLoading(false)
    }
  }

  const formatUptime = (s) => {
    const h = Math.floor(s / 3600)
    const m = Math.floor((s % 3600) / 60)
    const sec = s % 60
    return `${String(h).padStart(2,'0')}:${String(m).padStart(2,'0')}:${String(sec).padStart(2,'0')}`
  }

  const healthStatus = health?.status || 'checking'
  const statusClass = healthStatus === 'connected' ? 'status--ok' : healthStatus === 'degraded' ? 'status--warn' : 'status--err'
  const statusLabel = healthStatus === 'connected' ? 'Online' : healthStatus === 'degraded' ? 'Degraded' : healthStatus === 'checking' ? 'Checking...' : 'Offline'

  return (
    <div className="app">
      {/* Header */}
      <header className="header">
        <div className="header__badge">
          <span className="pulse"></span>
          SODS Protocol Dashboard
        </div>
        <h1 className="header__title">Behavioral Verification</h1>
        <p className="header__subtitle">
          Real-time on-chain behavioral symbol monitoring and trustless Merkle Tree verification
        </p>
      </header>

      {/* Stats Cards */}
      <section className="grid grid--4" style={{ marginBottom: '1.5rem' }}>
        <div className="card animate-in">
          <div className="card__header">
            <span className="card__title">Daemon Status</span>
            <span className="card__icon">🔗</span>
          </div>
          <div className={`status ${statusClass}`}>
            <span className="status__dot"></span>
            {statusLabel}
          </div>
          <p className="card__label">Last check: just now</p>
        </div>

        <div className="card animate-in">
          <div className="card__header">
            <span className="card__title">Session Uptime</span>
            <span className="card__icon">⏱️</span>
          </div>
          <div className="card__value">{formatUptime(uptime)}</div>
          <p className="card__label">Dashboard session</p>
        </div>

        <div className="card animate-in">
          <div className="card__header">
            <span className="card__title">Symbols</span>
            <span className="card__icon">🧬</span>
          </div>
          <div className="card__value">{SYMBOLS.length}</div>
          <p className="card__label">Supported behavioral types</p>
        </div>

        <div className="card animate-in">
          <div className="card__header">
            <span className="card__title">Mode</span>
            <span className="card__icon">🛡️</span>
          </div>
          <div className="card__value" style={{ fontSize: '1.4rem' }}>Trustless</div>
          <p className="card__label">Header-anchored BMT</p>
        </div>
      </section>

      {/* Main Content */}
      <div className="grid grid--2">
        {/* Verify Card */}
        <div className="card">
          <div className="card__header">
            <span className="card__title">Live Verification</span>
            <span className="card__icon">🔍</span>
          </div>

          <form className="verify-form" onSubmit={handleVerify}>
            <select
              className="verify-form__input"
              value={symbol}
              onChange={e => setSymbol(e.target.value)}
              id="symbol-select"
            >
              {SYMBOLS.map(s => (
                <option key={s.code} value={s.code}>{s.code} — {s.name}</option>
              ))}
            </select>
            <input
              className="verify-form__input"
              type="number"
              placeholder="Block number"
              value={block}
              onChange={e => setBlock(e.target.value)}
              id="block-input"
            />
            <select
              className="verify-form__input"
              value={chain}
              onChange={e => setChain(e.target.value)}
              id="chain-select"
              style={{ maxWidth: '140px' }}
            >
              <option value="sepolia">Sepolia</option>
              <option value="ethereum">Ethereum</option>
              <option value="base">Base</option>
              <option value="arbitrum">Arbitrum</option>
              <option value="optimism">Optimism</option>
            </select>
            <button
              className="verify-form__btn"
              type="submit"
              disabled={loading || !block}
              id="verify-btn"
            >
              {loading ? 'Verifying...' : 'Verify'}
            </button>
          </form>

          {verifyResult && (
            <div className={`result ${verifyResult.success !== false ? 'result--success' : 'result--error'}`}>
              {JSON.stringify(verifyResult, null, 2)}
            </div>
          )}

          <p className="card__label" style={{ marginTop: '1rem' }}>
            CLI equivalent: <code style={{ fontFamily: 'var(--font-mono)', color: 'var(--accent-cyan)' }}>
              sods verify {symbol} --block {block || '...'} --chain {chain}
            </code>
          </p>
        </div>

        {/* Symbols Table */}
        <div className="card">
          <div className="card__header">
            <span className="card__title">Symbol Dictionary</span>
            <span className="card__icon">📖</span>
          </div>
          <div style={{ maxHeight: '380px', overflowY: 'auto' }}>
            <table className="symbols-table">
              <thead>
                <tr>
                  <th>Symbol</th>
                  <th>Description</th>
                </tr>
              </thead>
              <tbody>
                {SYMBOLS.map(s => (
                  <tr key={s.code}>
                    <td className="sym">{s.code}</td>
                    <td>{s.name}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </div>

      {/* Footer */}
      <footer className="footer">
        SODS Protocol — Alpha (Research Prototype) · MIT OR Apache-2.0 ·{' '}
        <a href="https://github.com/logiccrafterdz/SODS-Protocol" target="_blank" rel="noopener noreferrer">
          GitHub
        </a>
      </footer>
    </div>
  )
}

export default App
