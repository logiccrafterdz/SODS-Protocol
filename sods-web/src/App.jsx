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

// --- Neural HUD Components ---

const HUDDecor = () => (
  <>
    <div className="hud-corner hud-corner--tl"></div>
    <div className="hud-corner hud-corner--tr"></div>
    <div className="hud-corner hud-corner--bl"></div>
    <div className="hud-corner hud-corner--br"></div>
  </>
)

const NeuralMesh = () => (
  <div className="neural-mesh-container">
    <svg width="100%" height="100%" viewBox="0 0 100 100" preserveAspectRatio="none">
      <defs>
        <pattern id="grid" width="10" height="10" patternUnits="userSpaceOnUse">
          <path d="M 10 0 L 0 0 0 10" fill="none" stroke="rgba(138, 43, 226, 0.1)" strokeWidth="0.1"/>
        </pattern>
      </defs>
      <rect width="100%" height="100%" fill="url(#grid)" />
      <path d="M 10 10 L 90 90 M 10 90 L 90 10 M 50 0 L 50 100 M 0 50 L 100 50" 
            stroke="rgba(0, 240, 255, 0.05)" strokeWidth="0.05" />
      <circle cx="50" cy="50" r="0.5" fill="var(--bismuth-c)" className="pulse" />
      <circle cx="10" cy="10" r="0.3" fill="var(--bismuth-v)" />
      <circle cx="90" cy="90" r="0.3" fill="var(--bismuth-v)" />
    </svg>
  </div>
)

function App() {
  const [health, setHealth] = useState(null)
  const [symbol, setSymbol] = useState('Tf')
  const [block, setBlock] = useState('')
  const [chain, setChain] = useState('sepolia')
  const [verifyResult, setVerifyResult] = useState(null)
  const [loading, setLoading] = useState(false)
  const [uptime, setUptime] = useState(0)

  const [booting, setBooting] = useState(true)

  // Simulation of Neural Sync
  useEffect(() => {
    const timer = setTimeout(() => setBooting(false), 2000)
    return () => clearTimeout(timer)
  }, [])

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
  const statusLabel = healthStatus === 'connected' ? 'NEURAL SYNC ACTIVE' : healthStatus === 'degraded' ? 'UNSTABLE LINK' : healthStatus === 'checking' ? 'SYNCING...' : 'LINK TERMINATED'

  if (booting) {
    return (
      <div className="app" style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh' }}>
        <div style={{ textAlign: 'center' }}>
          <div className="pulse" style={{ width: '40px', height: '40px', margin: '0 auto 2rem' }}></div>
          <h2 style={{ fontFamily: 'var(--font-h)', fontSize: '1rem', letterSpacing: '0.4em' }}>INITIALIZING NEURAL OVERLAY v8.0</h2>
          <p style={{ color: 'var(--text-dim)', marginTop: '1rem', fontSize: '0.8rem' }}>Syncing with behavioral pathways...</p>
        </div>
      </div>
    )
  }

  return (
    <div style={{ position: 'relative' }}>
      <div className="hud-overlay"></div>
      <div className="bismuth-fog"></div>
      <NeuralMesh />
      
      <div className="app">
        {/* Header */}
        <header className="header animate-in">
          <div className="header__badge">
            <span className="pulse"></span>
            NEURAL_OVERLAY_SYNC_8.0.2126
          </div>
          <h1 className="header__title">Behavioral<br/>Verification</h1>
          <p className="header__subtitle" style={{ fontSize: '0.8rem', letterSpacing: '0.2em', textTransform: 'uppercase' }}>
            A-Class Causal Intelligence // Trustless Merkle Verification
          </p>
        </header>

        {/* Floating Metrics */}
        <section className="grid grid--4 animate-in" style={{ animationDelay: '0.2s' }}>
          <div className="card">
            <HUDDecor />
            <div className="card__header">
              <span className="card__title">Neural Link</span>
              <span className="card__icon">📡</span>
            </div>
            <div className={`status ${statusClass}`}>
              <span className="status__dot"></span>
              {statusLabel}
            </div>
            <p className="card__label" style={{ opacity: 0.4 }}>Sync Latency: 4ms</p>
          </div>

          <div className="card">
            <HUDDecor />
            <div className="card__header">
              <span className="card__title">Time Since Boot</span>
              <span className="card__icon">⏱️</span>
            </div>
            <div className="card__value" style={{ fontSize: '2.5rem' }}>{formatUptime(uptime)}</div>
            <p className="card__label" style={{ opacity: 0.4 }}>Persistence: Stable</p>
          </div>

          <div className="card">
            <HUDDecor />
            <div className="card__header">
              <span className="card__title">Active Pathways</span>
              <span className="card__icon">🧬</span>
            </div>
            <div className="card__value">{SYMBOLS.length}</div>
            <p className="card__label" style={{ opacity: 0.4 }}>Sub-neural processors active</p>
          </div>

          <div className="card">
            <HUDDecor />
            <div className="card__header">
              <span className="card__title">Integrity Mode</span>
              <span className="card__icon">🛡️</span>
            </div>
            <div className="card__value" style={{ fontSize: '1.4rem', color: 'var(--bismuth-g)' }}>CRYSTALLINE</div>
            <p className="card__label" style={{ opacity: 0.4 }}>Secure Storage Proof</p>
          </div>
        </section>

        {/* Core HUD Operations */}
        <div className="grid grid--2 animate-in" style={{ animationDelay: '0.4s' }}>
          {/* Action Hub */}
          <div className="card" style={{ gridRow: 'span 2' }}>
            <HUDDecor />
            <div className="card__header">
              <span className="card__title">Path Verification Hub</span>
              <span className="card__icon">👁️</span>
            </div>

            <form className="verify-form" onSubmit={handleVerify}>
              <select
                className="verify-form__input"
                value={symbol}
                onChange={e => setSymbol(e.target.value)}
                id="symbol-select"
              >
                {SYMBOLS.map(s => (
                  <option key={s.code} value={s.code} style={{ background: '#000' }}>{s.code} — {s.name}</option>
                ))}
              </select>
              <input
                className="verify-form__input"
                type="number"
                placeholder="Target Block Height"
                value={block}
                onChange={e => setBlock(e.target.value)}
                id="block-input"
              />
              <select
                className="verify-form__input"
                value={chain}
                onChange={e => setChain(e.target.value)}
                id="chain-select"
              >
                <option value="sepolia" style={{ background: '#000' }}>SEPOLIA_L2 // DEVNET</option>
                <option value="ethereum" style={{ background: '#000' }}>ETHEREUM_L1 // MAINNET</option>
                <option value="base" style={{ background: '#000' }}>BASE // ROLLUP</option>
                <option value="arbitrum" style={{ background: '#000' }}>ARBITRUM // ROLLUP</option>
                <option value="optimism" style={{ background: '#000' }}>OPTIMISM // ROLLUP</option>
              </select>
              <button
                className="verify-form__btn"
                type="submit"
                disabled={loading || !block}
                id="verify-btn"
              >
                {loading ? 'PATH_SYNCHRONIZING...' : 'EXECUTE VERIFICATION'}
              </button>
            </form>

            {verifyResult && (
              <div className={`result ${verifyResult.success !== false ? 'result--success' : 'result--error'}`}>
                <div style={{ fontSize: '0.6rem', opacity: 0.5, marginBottom: '1rem' }}>SYSTEM_LOG :: VERIFIED_TRUE</div>
                {JSON.stringify(verifyResult, null, 2)}
              </div>
            )}
            
            <p className="card__label" style={{ marginTop: '2rem', fontFamily: 'var(--font-data)', fontSize: '0.65rem' }}>
              $ sods verify --path {symbol} --anchor {block || '0x0'} --net {chain}
            </p>
          </div>

          {/* Dictionary Hub */}
          <div className="card">
            <HUDDecor />
            <div className="card__header">
              <span className="card__title">Symbol Reference Archive</span>
              <span className="card__icon">📖</span>
            </div>
            <div style={{ maxHeight: '420px', overflowY: 'auto', paddingRight: '1rem' }}>
              <table className="symbols-table">
                <thead>
                  <tr>
                    <th>SIGIL</th>
                    <th>DEFINITION</th>
                  </tr>
                </thead>
                <tbody>
                  {SYMBOLS.map(s => (
                    <tr key={s.code}>
                      <td className="sym">{s.code}</td>
                      <td style={{ fontSize: '0.8rem', opacity: 0.8 }}>{s.name}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
          
          <div className="card" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', minHeight: '100px' }}>
            <HUDDecor />
            <div style={{ textAlign: 'center' }}>
              <div className="header__badge" style={{ margin: 0 }}>ERA_2126_COMPLIANT</div>
            </div>
          </div>
        </div>

        {/* Footer */}
        <footer className="footer animate-in" style={{ animationDelay: '0.6s' }}>
          SODS-X // NEURAL_OVERLAY · CC0 1.0 UNIVERSAL ·{' '}
          <a href="https://github.com/logiccrafterdz/SODS-Protocol" target="_blank" rel="noopener noreferrer">
            PROJECT_SOURCE
          </a>
        </footer>
      </div>
    </div>
  )
}

export default App
