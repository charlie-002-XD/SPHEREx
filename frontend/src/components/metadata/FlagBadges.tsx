// Flag badges: compact chips showing flag names exactly as delivered by the
// API. Bit meanings are never hardcoded — when a flag_legend is supplied and
// a name matches one of its descriptions, the tooltip exposes the bit number.
import "./metadata.css";

export interface FlagBadgesProps {
  flags: readonly string[];
  // Optional bit-number -> description map from FramesResponse.flag_legend.
  legend?: Record<string, string>;
}

// Returns the bit whose description equals `name`, or null. The name->bit
// mapping is derived at render time from the API payload, not hardcoded.
export function flagBitFor(
  legend: Record<string, string> | undefined,
  name: string,
): string | null {
  if (legend === undefined) return null;
  for (const [bit, desc] of Object.entries(legend)) {
    if (desc === name) return bit;
  }
  return null;
}

export function FlagBadges({ flags, legend }: FlagBadgesProps) {
  if (flags.length === 0) return null;
  return (
    <ul className="md-badges" aria-label="Quality flags">
      {flags.map((f, i) => {
        const bit = flagBitFor(legend, f);
        return (
          <li
            key={`${f}-${i}`}
            className="md-badge"
            title={bit === null ? f : `${f} (bit ${bit})`}
          >
            {f}
          </li>
        );
      })}
    </ul>
  );
}
