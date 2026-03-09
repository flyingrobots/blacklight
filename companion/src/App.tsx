import React, { useState, useEffect, useRef } from 'react';
import { Play, Square, ExternalLink, ChevronDown, ChevronRight, Activity, Terminal, Info, AlertCircle } from 'lucide-react';

interface IndexerStatus {
  status: string;
  outdated_count: number;
  progress: {
    phase: string;
    files_total: number;
    files_done: number;
    messages_processed: number;
  };
}

interface WorkerStatus {
  status: string;
  sessions_total: number;
  sessions_done: number;
  sessions_failed: number;
  outdated_count: number;
}

interface OverviewStats {
  total_sessions: number;
  total_messages: number;
  last_session: string | null;
}

declare global {
  interface Window {
    electron: {
      openDashboard: () => Promise<void>;
      startServer: () => Promise<void>;
      stopServer: () => Promise<void>;
      getServerStatus: () => Promise<string>;
      onServerStatus: (callback: (status: string) => void) => () => void;
    };
  }
}

const BASE_URL = 'http://localhost:3141/api';

const LogViewer = ({ lines }: { lines: string[] }) => {
  const scrollRef = useRef<HTMLDivElement>(null);
  
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [lines]);

  return (
    <div className="log-viewer" ref={scrollRef}>
      {lines.length === 0 ? (
        <div className="status-text">No recent logs...</div>
      ) : (
        lines.map((l, i) => <div key={i} className="log-line">{l}</div>)
      )}
    </div>
  );
};

function App() {
  const [serverStatus, setServerStatus] = useState<'running' | 'stopped'>('stopped');
  const [indexer, setIndexer] = useState<IndexerStatus | null>(null);
  const [enricher, setEnricher] = useState<WorkerStatus | null>(null);
  const [classifier, setClassifier] = useState<WorkerStatus | null>(null);
  const [overview, setOverview] = useState<OverviewStats | null>(null);
  const [indexerLogs, setIndexerLogs] = useState<string[]>([]);
  const [enricherLogs, setEnricherLogs] = useState<string[]>([]);
  
  const [expanded, setExpanded] = useState<Record<string, boolean>>({ indexer: true, enrichment: false, classifier: false });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [alerts, setAlerts] = useState<{ id: number; msg: string; type: 'info' | 'error' }[]>([]);

  const prevStatus = useRef<Record<string, string>>({});

  useEffect(() => {
    window.electron.getServerStatus().then((s) => setServerStatus(s as any));
    const unsubscribe = window.electron.onServerStatus((status) => {
      setServerStatus(status as any);
      if (status === 'running') addAlert('Server started');
      else addAlert('Server stopped', 'error');
    });
    return unsubscribe;
  }, []);

  const addAlert = (msg: string, type: 'info' | 'error' = 'info') => {
    const id = Date.now();
    setAlerts(prev => [...prev, { id, msg, type }]);
    setTimeout(() => {
      setAlerts(prev => prev.filter(a => a.id !== id));
    }, 4000);
  };

  const fetchStatus = async () => {
    if (serverStatus !== 'running') {
      setLoading(false);
      return;
    }

    try {
      const [idxRes, enrRes, clsRes, ovRes, idxLogRes, enrLogRes] = await Promise.all([
        fetch(`${BASE_URL}/indexer/status`),
        fetch(`${BASE_URL}/enrichment/status`),
        fetch(`${BASE_URL}/classifier/status`),
        fetch(`${BASE_URL}/analytics/overview`),
        fetch(`${BASE_URL}/indexer/logs`),
        fetch(`${BASE_URL}/enrichment/logs`),
      ]);

      const idxData = await idxRes.json();
      const enrData = await enrRes.json();
      const clsData = await clsRes.json();
      const ovData = await ovRes.json();
      const idxLogs = await idxLogRes.json();
      const enrLogs = await enrLogRes.json();

      setIndexer(idxData);
      setEnricher(enrData);
      setClassifier(clsData);
      setOverview(ovData);
      setIndexerLogs(idxLogs);
      setEnricherLogs(enrLogs);

      checkTransitions('Indexer', idxData.status);
      checkTransitions('Enrichment', enrData.status);
      checkTransitions('Classification', clsData.status);

      setError(null);
    } catch (err) {
      if (serverStatus === 'running') setError('Backend unreachable');
    } finally {
      setLoading(false);
    }
  };

  const checkTransitions = (label: string, status: string) => {
    const last = prevStatus.current[label];
    if (last === 'running' && status === 'completed') addAlert(`${label} finished!`);
    if (last === 'running' && status === 'failed') addAlert(`${label} failed`, 'error');
    prevStatus.current[label] = status;
  };

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 2000);
    return () => clearInterval(interval);
  }, [serverStatus]);

  const handleAction = async (type: string, action: string, params = {}) => {
    try {
      await fetch(`${BASE_URL}/${type}/${action}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(params),
      });
      fetchStatus();
    } catch (err) { console.error(err); }
  };

  const toggle = (key: string) => setExpanded(prev => ({ ...prev, [key]: !prev[key] }));

  return (
    <div className="container">
      <header className="header">
        <div className="brand-group">
          <div className={`status-pip ${serverStatus}`} title={`Server is ${serverStatus}`}></div>
          <div className="brand">Blacklight</div>
        </div>
        <div className="controls">
          {serverStatus === 'stopped' ? (
            <button className="btn btn-primary" onClick={() => window.electron.startServer()}>Start Server</button>
          ) : (
            <button className="btn btn-danger-text" onClick={() => window.electron.stopServer()}>Stop Server</button>
          )}
          <button className="btn-link" title="Web Dashboard" onClick={() => window.electron.openDashboard()}>
            <ExternalLink size={14} />
          </button>
        </div>
      </header>

      {serverStatus === 'running' && overview && (
        <div className="mini-stats">
          <div className="stat-box">
            <span className="stat-val">{overview.total_sessions}</span>
            <span className="stat-lab">Sessions</span>
          </div>
          <div className="stat-box">
            <span className="stat-val">{overview.total_messages.toLocaleString()}</span>
            <span className="stat-lab">Messages</span>
          </div>
        </div>
      )}

      <div className="accordion">
        {serverStatus === 'running' ? (
          <>
            {/* Indexer */}
            <div className="acc-item">
              <div className="acc-header" onClick={() => toggle('indexer')}>
                <div className="acc-title-group">
                  <Activity size={14} style={{ color: indexer?.status === 'running' ? 'var(--bl-success)' : 'inherit' }} />
                  <span className="acc-title">Indexer</span>
                </div>
                {expanded.indexer ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              </div>
              {expanded.indexer && (
                <div className="acc-content">
                  <div className="sc-header">
                    <span className="status-text">{indexer?.status || 'idle'}</span>
                    {indexer?.status === 'running' && <span className="status-text">{Math.round((indexer.progress.files_done / (indexer.progress.files_total || 1)) * 100)}%</span>}
                  </div>
                  {indexer?.status === 'running' && (
                    <div className="progress-bar"><div className="progress-fill" style={{ width: `${(indexer.progress.files_done / (indexer.progress.files_total || 1)) * 100}%` }}></div></div>
                  )}
                  {indexer && indexer.outdated_count > 0 && indexer.status !== 'running' && (
                    <div className="outdated-text">{indexer.outdated_count} sessions to index</div>
                  )}
                  <div className="controls">
                    {indexer?.status === 'running' ? (
                      <button className="btn btn-danger-text" onClick={() => handleAction('indexer', 'stop')}>Stop</button>
                    ) : (
                      <button className="btn btn-primary" onClick={() => handleAction('indexer', 'start')}>Start</button>
                    )}
                  </div>
                  <LogViewer lines={indexerLogs} />
                </div>
              )}
            </div>

            {/* Enrichment */}
            <div className="acc-item">
              <div className="acc-header" onClick={() => toggle('enrichment')}>
                <div className="acc-title-group">
                  <Terminal size={14} style={{ color: enricher?.status === 'running' ? 'var(--bl-success)' : 'inherit' }} />
                  <span className="acc-title">Enrichment</span>
                </div>
                {expanded.enrichment ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              </div>
              {expanded.enrichment && (
                <div className="acc-content">
                  <div className="sc-header">
                    <span className="status-text">{enricher?.status || 'idle'}</span>
                    {enricher?.status === 'running' && <span className="status-text">{Math.round((enricher.sessions_done / (enricher.sessions_total || 1)) * 100)}%</span>}
                  </div>
                  {enricher?.status === 'running' && (
                    <div className="progress-bar"><div className="progress-fill" style={{ width: `${(enricher.sessions_done / (enricher.sessions_total || 1)) * 100}%` }}></div></div>
                  )}
                  {enricher && enricher.outdated_count > 0 && enricher.status !== 'running' && (
                    <div className="outdated-text">{enricher.outdated_count} pending</div>
                  )}
                  <div className="controls">
                    {enricher?.status === 'running' ? (
                      <button className="btn btn-danger-text" onClick={() => handleAction('enrichment', 'stop')}>Stop</button>
                    ) : (
                      <button className="btn btn-primary" onClick={() => handleAction('enrichment', 'start')}>Start</button>
                    )}
                  </div>
                  <LogViewer lines={enricherLogs} />
                </div>
              )}
            </div>

            {/* Classification */}
            <div className="acc-item">
              <div className="acc-header" onClick={() => toggle('classifier')}>
                <div className="acc-title-group">
                  <Info size={14} style={{ color: classifier?.status === 'running' ? 'var(--bl-success)' : 'inherit' }} />
                  <span className="acc-title">Classification</span>
                </div>
                {expanded.classifier ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              </div>
              {expanded.classifier && (
                <div className="acc-content">
                  <div className="sc-header">
                    <span className="status-text">{classifier?.status || 'idle'}</span>
                    {classifier?.status === 'running' && <span className="status-text">{Math.round((classifier.sessions_done / (classifier.sessions_total || 1)) * 100)}%</span>}
                  </div>
                  {classifier?.status === 'running' && (
                    <div className="progress-bar"><div className="progress-fill" style={{ width: `${(classifier.sessions_done / (classifier.sessions_total || 1)) * 100}%` }}></div></div>
                  )}
                  {classifier && classifier.outdated_count > 0 && classifier.status !== 'running' && (
                    <div className="outdated-text">{classifier.outdated_count} pending</div>
                  )}
                  <div className="controls">
                    {classifier?.status === 'running' ? (
                      <button className="btn btn-danger-text" onClick={() => handleAction('classifier', 'stop')}>Stop</button>
                    ) : (
                      <button className="btn btn-primary" onClick={() => handleAction('classifier', 'start')}>Start</button>
                    )}
                  </div>
                </div>
              )}
            </div>
          </>
        ) : (
          <div className="empty-state">
            <AlertCircle size={24} style={{ opacity: 0.3, marginBottom: '0.5rem' }} />
            <span>Connect to start monitoring</span>
          </div>
        )}
      </div>

      <div className="alerts-zone">
        {alerts.map(a => (
          <div key={a.id} className={`alert-toast ${a.type}`}>{a.msg}</div>
        ))}
      </div>

      <footer className="footer">
        <div className="status-text">v0.1.0</div>
        {overview?.last_session && <div className="status-text">Last: {new Date(overview.last_session).toLocaleDateString()}</div>}
      </footer>
    </div>
  );
}

export default App;
