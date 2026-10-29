export interface Step {
  name: string;
  state?: 'done' | 'now' | 'fail' | 'todo';
  note?: string;
}

export interface StepListProps {
  steps: Step[];
}

export function StepList({ steps }: StepListProps) {
  return (
    <ol className="tf-steps">
      {steps.map(({ name, state = 'todo', note }) => (
        <li key={name} className={`tf-step tf-step-${state}`} aria-current={state === 'now' ? 'step' : undefined}>
          <span className="tf-step-dot" aria-hidden="true" />
          <span>{name}</span>
          {note && <span className="tf-step-note">{note}</span>}
        </li>
      ))}
    </ol>
  );
}
