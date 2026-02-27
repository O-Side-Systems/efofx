import { useState } from 'react';
import type { EstimationOutput } from '../types/widget';

interface EstimateCardProps {
  estimate: EstimationOutput;
}

const fmt = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 0,
});

/**
 * EstimateCard — P50/P80 range bar visualization and cost breakdown accordion.
 *
 * Range bar: horizontal bar showing P50 ("Most likely") and P80 ("High end") positions.
 * Bar fill uses var(--brand-accent).
 *
 * Accordion: each CostCategoryEstimate row is expandable.
 * Collapsed: category name + P50 subtotal.
 * Expanded: P50, P80, and percentage of total.
 */
export function EstimateCard({ estimate }: EstimateCardProps) {
  const [expandedRows, setExpandedRows] = useState<Set<string>>(new Set());

  function toggleRow(category: string) {
    setExpandedRows(prev => {
      const next = new Set(prev);
      if (next.has(category)) {
        next.delete(category);
      } else {
        next.add(category);
      }
      return next;
    });
  }

  // P50 is always 100% of the bar base; P80 is proportionally positioned
  const p50 = estimate.total_cost_p50;
  const p80 = estimate.total_cost_p80;
  // P50 marker sits at 55% of bar width; P80 at 85% — gives visual separation
  const p50Pct = 55;
  const p80Pct = 85;

  return (
    <div className="efofx-estimate-card">
      {/* Range summary */}
      <div className="efofx-estimate-summary">
        <p className="efofx-estimate-label">Your Project Estimate</p>
        <div className="efofx-estimate-totals">
          <div className="efofx-estimate-total-item">
            <span className="efofx-estimate-total-value">{fmt.format(p50)}</span>
            <span className="efofx-estimate-total-label">Most likely</span>
          </div>
          <div className="efofx-estimate-total-separator">—</div>
          <div className="efofx-estimate-total-item">
            <span className="efofx-estimate-total-value">{fmt.format(p80)}</span>
            <span className="efofx-estimate-total-label">High end</span>
          </div>
        </div>
      </div>

      {/* Range bar */}
      <div className="efofx-range-bar-container" aria-hidden="true">
        <div className="efofx-range-bar">
          <div className="efofx-range-bar-fill" />
          <div className="efofx-range-marker" style={{ left: `${p50Pct}%` }}>
            <div className="efofx-range-marker-dot" />
            <span className="efofx-range-label efofx-range-label--p50">{fmt.format(p50)}</span>
          </div>
          <div className="efofx-range-marker" style={{ left: `${p80Pct}%` }}>
            <div className="efofx-range-marker-dot" />
            <span className="efofx-range-label efofx-range-label--p80">{fmt.format(p80)}</span>
          </div>
        </div>
      </div>

      {/* Timeline */}
      <p className="efofx-estimate-timeline">
        Timeline: {estimate.timeline_weeks_p50}–{estimate.timeline_weeks_p80} weeks
      </p>

      {/* Cost breakdown accordion */}
      {estimate.cost_breakdown.length > 0 && (
        <div className="efofx-accordion">
          <p className="efofx-accordion-title">Cost Breakdown</p>
          {estimate.cost_breakdown.map(row => {
            const isOpen = expandedRows.has(row.category);
            return (
              <div key={row.category} className="efofx-accordion-row">
                <button
                  className="efofx-accordion-header"
                  onClick={() => toggleRow(row.category)}
                  type="button"
                  aria-expanded={isOpen}
                >
                  <span className="efofx-accordion-category">{row.category}</span>
                  <span className="efofx-accordion-amount">{fmt.format(row.p50_cost)}</span>
                  <span className={`efofx-accordion-chevron${isOpen ? ' efofx-accordion-chevron--open' : ''}`}>
                    ▾
                  </span>
                </button>
                {isOpen && (
                  <div className="efofx-accordion-expanded">
                    <div className="efofx-accordion-detail">
                      <span>Most likely</span>
                      <span>{fmt.format(row.p50_cost)}</span>
                    </div>
                    <div className="efofx-accordion-detail">
                      <span>High end</span>
                      <span>{fmt.format(row.p80_cost)}</span>
                    </div>
                    <div className="efofx-accordion-detail">
                      <span>% of total</span>
                      <span>{row.percentage_of_total.toFixed(1)}%</span>
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

export default EstimateCard;
