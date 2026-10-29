export interface StageTrackProps {
  current: number;
  total?: number;
  stage?: string;
  detail?: string;
  failed?: boolean;
}

export function StageTrack({ current, total = 8, stage, detail, failed }: StageTrackProps) {
  const label = `${stage ? `${stage} ` : ''}${current}/${total}${failed ? ', failed' : ''}`;
  const fill = total > 1 ? Math.max(0, Math.min(1, (current - 1) / (total - 1))) : 0;
  return (
    <div className="tf-track" role="img" aria-label={label}>
      <div className="tf-track-bar">
        <div className="tf-track-rail" />
        <div className="tf-track-fill" style={{ width: `calc((100% - 8px) * ${fill})` }} />
        <div className="tf-track-dots">
          {Array.from({ length: total }, (_, i) => {
            const n = i + 1;
            const cls = n < current ? 'tf-dot-done' : n === current ? `tf-dot-now${failed ? ' tf-dot-fail' : ''}` : '';
            return <span key={n} className={`tf-dot ${cls}`.trim()} />;
          })}
        </div>
      </div>
      {(stage || detail) && (
        <div className="tf-track-caption">
          {stage && <b>{stage}</b>}{stage && detail ? ' · ' : ''}{detail}
        </div>
      )}
    </div>
  );
}
