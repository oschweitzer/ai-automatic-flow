import { cleanup, render, screen, fireEvent } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ActionCard, Avatar, Chip, NavItem, Section, StageTrack, StatusPill } from './index';

afterEach(cleanup);

describe('ui', () => {
  it('chip_exposes_pressed', () => {
    render(<Chip selected>Sprint 12</Chip>);
    expect(screen.getByRole('button', { name: 'Sprint 12' }).getAttribute('aria-pressed')).toBe('true');
  });

  it('avatar_has_accessible_name', () => {
    render(<Avatar initials="AL" label="Alice Lambert" />);
    const b = screen.getByRole('button', { name: 'Alice Lambert' });
    expect(b.getAttribute('title')).toBe('Alice Lambert');
    expect(b.getAttribute('aria-pressed')).toBe('false');
  });

  it('stage_track_label', () => {
    render(<StageTrack current={5} stage="ci-gate" failed />);
    expect(screen.getByLabelText('ci-gate 5/8, failed')).toBeTruthy();
  });

  it('status_pill_always_has_text', () => {
    render(<StatusPill tone="you">toi · plan à valider</StatusPill>);
    expect(screen.getByText('toi · plan à valider')).toBeTruthy();
  });

  it('section_toggles', () => {
    const onToggle = vi.fn();
    render(<Section title="En cours" count={2} open={false} onToggle={onToggle}><p>x</p></Section>);
    fireEvent.click(screen.getByText('En cours'));
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it('nav_item_hides_zero_count', () => {
    const { rerender } = render(<NavItem label="Shop" count={0} />);
    expect(screen.queryByText('0')).toBeNull();
    rerender(<NavItem label="Shop" count={3} />);
    expect(screen.getByText('3')).toBeTruthy();
  });

  it('action_card_runs_action', () => {
    const onAction = vi.fn();
    render(<ActionCard title="À toi" actionLabel="Ouvrir le premier" onAction={onAction} />);
    fireEvent.click(screen.getByRole('button', { name: 'Ouvrir le premier' }));
    expect(onAction).toHaveBeenCalled();
  });
});
