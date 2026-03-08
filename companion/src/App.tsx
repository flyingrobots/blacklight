import React, { useState, useEffect } from 'react';
import { Play, Square, Pause, ExternalLink, RefreshCw } from 'lucide-react';

interface IndexerStatus {
  status: string;
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
}

declare global {
  interface Window {
    electron: {
      openDashboard: () => Promise<void>;
    };
  }
}

const BASE_URL = 'http://localhost:3141/api';

function App() {
  const [indexer, setIndexer] = useState<IndexerStatus | null>(null);
  const [enricher, setEnricher] = useState<WorkerStatus | null>(null);
  const [classifier, setClassifier] = useState<WorkerStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchStatus = async () => {
    try {
      const [idxRes, enrRes, clsRes] = await Promise.all([
        fetch(`${BASE_URL}/indexer/status`),
        fetch(`${BASE_URL}/enrichment/status`),
        fetch(`${BASE_URL}/classifier/status`),
      ]);

      if (!idxRes.ok || !enrRes.ok || !clsRes.ok) throw new Error('Server offline');

      const idxData = await idxRes.json();
      const enrData = await enrRes.json();
      const clsData = await clsRes.json();

      setIndexer(idxData);
      setEnricher(enrData);
      setClassifier(clsData);
      setError(null);
    } catch (err) {
      setError('Could not connect to Blacklight server');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 2000);
    return () => clearInterval(interval);
  }, []);

  const handleAction = async (type: string, action: string, params = {}) => {
    try {
      await fetch(`${BASE_URL}/${type}/${action}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(params),
      });
      fetchStatus();
    } catch (err) {
      console.error(err);
    }
  };

  if (loading) {
    return <div className="container"><div className="progress-text">Connecting...</div></div>;
  }

  if (error) {
    return (
      <div className="container">
        <div className="brand">Blacklight</div>
        <div className="error-msg">{error}</div>
        <button className="btn btn-primary" onClick={fetchStatus}>
          <RefreshCw size={14} /> Retry
        </button>
      </div>
    );
  }

  const getIndexerPct = () => {
    if (!indexer) return 0;
    const { files_total, files_done } = indexer.progress;
    return files_total === 0 ? 0 : (files_done / files_total) * 100;
  };

  const getWorkerPct = (w: WorkerStatus | null) => {
    if (!w) return 0;
    return w.sessions_total === 0 ? 0 : (w.sessions_done / w.sessions_total) * 100;
  };

  return (
    <div className="container">
      <header className="header">
        <div className="brand">Blacklight Companion</div>
        <button className="btn-link" onClick={() => window.electron.openDashboard()}>
          <ExternalLink size={14} />
        </button>
      </header>

      <div className="status-grid">
        {/* Indexer */}
        <div className="status-card">
          <div className="sc-header">
            <span className="sc-title">Indexer</span>
            <span className={`badge badge-${indexer?.status}`}>{indexer?.status}</span>
          </div>
          {indexer?.status === 'running' && (
            <div className="progress-section">
              <div className="progress-bar">
                <div className="progress-fill" style={{ width: `${getIndexerPct()}%` }}></div>
              </div>
              <div className="progress-text">{indexer.progress.phase} ({indexer.progress.files_done}/{indexer.progress.files_total})</div>
            </div>
          )}
          <div className="controls">
            {indexer?.status === 'running' ? (
              <>
                <button className="btn" onClick={() => handleAction('indexer', 'pause')}><Pause size={12} /> Pause</button>
                <button className="btn btn-danger" onClick={() => handleAction('indexer', 'stop')}><Square size={12} /> Stop</button>
              </>
            ) : indexer?.status === 'paused' ? (
              <button className="btn btn-primary" onClick={() => handleAction('indexer', 'resume')}><Play size={12} /> Resume</button>
            ) : (
              <button className="btn btn-primary" onClick={() => handleAction('indexer', 'start', { full: false })}><Play size={12} /> Start</button>
            )}
          </div>
        </div>

        {/* Enrichment */}
        <div className="status-card">
          <div className="sc-header">
            <span className="sc-title">Enrichment</span>
            <span className={`badge badge-${enricher?.status}`}>{enricher?.status}</span>
          </div>
          {enricher?.status === 'running' && (
            <div className="progress-section">
              <div className="progress-bar">
                <div className="progress-fill" style={{ width: `${getWorkerPct(enricher)}%` }}></div>
              </div>
              <div className="progress-text">{enricher.sessions_done}/{enricher.sessions_total} sessions</div>
            </div>
          )}
          <div className="controls">
            {enricher?.status === 'running' ? (
              <button className="btn btn-danger" onClick={() => handleAction('enrichment', 'stop')}><Square size={12} /> Stop</button>
            ) : (
              <button className="btn btn-primary" onClick={() => handleAction('enrichment', 'start', { force: false })}><Play size={12} /> Start</button>
            )}
          </div>
        </div>

        {/* Classifier */}
        <div className="status-card">
          <div className="sc-header">
            <span className="sc-title">Classification</span>
            <span className={`badge badge-${classifier?.status}`}>{classifier?.status}</span>
          </div>
          {classifier?.status === 'running' && (
            <div className="progress-section">
              <div className="progress-bar">
                <div className="progress-fill" style={{ width: `${getWorkerPct(classifier)}%` }}></div>
              </div>
              <div className="progress-text">{classifier.sessions_done}/{classifier.sessions_total} sessions</div>
            </div>
          )}
          <div className="controls">
            {classifier?.status === 'running' ? (
              <button className="btn btn-danger" onClick={() => handleAction('classifier', 'stop')}><Square size={12} /> Stop</button>
            ) : (
              <button className="btn btn-primary" onClick={() => handleAction('classifier', 'start', { force: false })}><Play size={12} /> Start</button>
            )}
          </div>
        </div>
      </div>

      <footer className="footer">
        <button className="btn-link" onClick={() => window.electron.openDashboard()}>Open Dashboard</button>
        <div className="progress-text">v0.1.0</div>
      </footer>
    </div>
  );
}

export default App;
