import type { AddonHealthCard } from "../lib/plannedAddons";

export function AddonHealthStrip({ cards }: { cards: AddonHealthCard[] }) {
  return (
    <div className="addons__health-strip" aria-label="Add-on health">
      {cards.map((card) => (
        <section
          className={`addons__health-card addons__health-card--${card.tone}`}
          key={card.id}
        >
          <div className="addons__health-heading">
            <strong>{card.name}</strong>
            <span>{card.statusLabel}</span>
          </div>
          <p>{card.detail}</p>
          <ul>
            {card.evidence.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
          <div
            className="addons__health-trend"
            aria-label={`${card.name} health trend`}
          >
            <div className="addons__health-trend-heading">
              <span>{card.trend.label}</span>
              <strong>{card.trend.value}</strong>
            </div>
            {card.trend.points.length > 0 ? (
              <div className="addons__health-sparkline" aria-hidden="true">
                {card.trend.points.map((point) => {
                  const maxValue = Math.max(
                    ...card.trend.points.map((item) => item.value),
                    1,
                  );
                  const height = Math.max(12, (point.value / maxValue) * 100);
                  return (
                    <span
                      key={`${point.label}-${point.value}`}
                      style={{ height: `${height}%` }}
                      title={`${point.label}: ${point.value.toLocaleString()}`}
                    />
                  );
                })}
              </div>
            ) : null}
            <small>{card.trend.detail}</small>
          </div>
          <em>{card.nextAction}</em>
        </section>
      ))}
    </div>
  );
}
