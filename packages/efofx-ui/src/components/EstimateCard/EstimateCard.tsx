import { useState } from 'react';
import type { EstimationOutput } from '../../types/estimation';
import styles from './EstimateCard.module.css';

export type { EstimationOutput } from '../../types/estimation';

export interface EstimateCardProps {
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
    <div className={styles.estimateCard}>
      {/* Range summary */}
      <div className={styles.estimateSummary}>
        <p className={styles.estimateLabel}>Your Project Estimate</p>
        <div className={styles.estimateTotals}>
          <div className={styles.estimateTotalItem}>
            <span className={styles.estimateTotalValue}>{fmt.format(p50)}</span>
            <span className={styles.estimateTotalLabel}>Most likely</span>
          </div>
          <div className={styles.estimateTotalSeparator}>—</div>
          <div className={styles.estimateTotalItem}>
            <span className={styles.estimateTotalValue}>{fmt.format(p80)}</span>
            <span className={styles.estimateTotalLabel}>High end</span>
          </div>
        </div>
      </div>

      {/* Range bar */}
      <div className={styles.rangeBarContainer} aria-hidden="true">
        <div className={styles.rangeBar}>
          <div className={styles.rangeBarFill} />
          <div className={styles.rangeMarker} style={{ left: `${p50Pct}%` }}>
            <div className={styles.rangeMarkerDot} />
            <span className={`${styles.rangeLabel} ${styles.rangeLabelP50}`}>{fmt.format(p50)}</span>
          </div>
          <div className={styles.rangeMarker} style={{ left: `${p80Pct}%` }}>
            <div className={styles.rangeMarkerDot} />
            <span className={`${styles.rangeLabel} ${styles.rangeLabelP80}`}>{fmt.format(p80)}</span>
          </div>
        </div>
      </div>

      {/* Timeline */}
      <p className={styles.estimateTimeline}>
        Timeline: {estimate.timeline_weeks_p50}–{estimate.timeline_weeks_p80} weeks
      </p>

      {/* Cost breakdown accordion */}
      {estimate.cost_breakdown.length > 0 && (
        <div className={styles.accordion}>
          <p className={styles.accordionTitle}>Cost Breakdown</p>
          {estimate.cost_breakdown.map(row => {
            const isOpen = expandedRows.has(row.category);
            return (
              <div key={row.category} className={styles.accordionRow}>
                <button
                  className={styles.accordionHeader}
                  onClick={() => toggleRow(row.category)}
                  type="button"
                  aria-expanded={isOpen}
                >
                  <span className={styles.accordionCategory}>{row.category}</span>
                  <span className={styles.accordionAmount}>{fmt.format(row.p50_cost)}</span>
                  <span className={isOpen ? `${styles.accordionChevron} ${styles.accordionChevronOpen}` : styles.accordionChevron}>
                    ▾
                  </span>
                </button>
                {isOpen && (
                  <div className={styles.accordionExpanded}>
                    <div className={styles.accordionDetail}>
                      <span>Most likely</span>
                      <span>{fmt.format(row.p50_cost)}</span>
                    </div>
                    <div className={styles.accordionDetail}>
                      <span>High end</span>
                      <span>{fmt.format(row.p80_cost)}</span>
                    </div>
                    <div className={styles.accordionDetail}>
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
