import React from 'react';

type StatusType = 'queued' | 'running' | 'success' | 'failed' | 'idle' | string;

interface StatusChipProps {
  status: StatusType;
  label?: string;
  className?: string;
}

export const StatusChip: React.FC<StatusChipProps> = ({ status, label, className = '' }) => {
  const getStatusClass = (s: string) => {
    switch (s.toLowerCase()) {
      case 'queued':
      case 'pending':
        return 'status-queued';
      case 'running':
      case 'processing':
      case 'recording':
        return 'status-running';
      case 'success':
      case 'completed':
      case 'finished':
        return 'status-success';
      case 'failed':
      case 'error':
        return 'status-failed';
      default:
        return 'status-fallback';
    }
  };

  const displayLabel = label || status.charAt(0).toUpperCase() + status.slice(1);
  const isRecording = status.toLowerCase() === 'recording';

  return (
    <div className={`status-chip ${getStatusClass(status)} ${className}`}>
      {isRecording && (
        <span className="pulse-red status-dot" />
      )}
      {displayLabel}
    </div>
  );
};
