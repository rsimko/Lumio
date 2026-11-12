/** Coordinate formatting and validation shared by the location editors. */

export interface MapPosition {
  /** Horizontal position as a percentage of the placeholder map. */
  left: number;
  /** Vertical position as a percentage of the placeholder map. */
  top: number;
}

function finite(value: number | null | undefined): value is number {
  return value != null && Number.isFinite(value);
}

export function formatLatitude(value: number | null | undefined): string {
  if (!finite(value)) return '';
  return `${Math.abs(value).toFixed(5)}° ${value >= 0 ? 'N' : 'S'}`;
}

export function formatLongitude(value: number | null | undefined): string {
  if (!finite(value)) return '';
  return `${Math.abs(value).toFixed(5)}° ${value >= 0 ? 'E' : 'W'}`;
}

export function formatCoordinates(
  latitude: number | null | undefined,
  longitude: number | null | undefined,
): string {
  const lat = formatLatitude(latitude);
  const lon = formatLongitude(longitude);
  if (!lat && !lon) return '';
  return [lat, lon].filter(Boolean).join(', ');
}

export function formatAltitude(value: number | null | undefined): string {
  if (!finite(value)) return '';
  const metres = Math.abs(value);
  const rounded = metres >= 100 ? Math.round(metres) : Math.round(metres * 10) / 10;
  return `${rounded.toLocaleString()} m ${value < 0 ? 'below' : 'above'} sea level`;
}

/**
 * Parse decimal degrees or simple DMS input such as `48.8567 N`,
 * `48 51 24 N` or `48°51'24"N`. Returns null for blank or invalid values.
 */
export function parseCoordinate(raw: string, limit: number): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;

  const hemisphere = /([NSEWnsew])\s*$/.exec(trimmed)?.[1]?.toUpperCase() ?? null;
  const withoutHemisphere = hemisphere
    ? trimmed.slice(0, -1)
    : trimmed;
  const cleaned = withoutHemisphere.replace(/[^\d.+-]+/g, ' ').trim();
  if (!cleaned) return null;

  const parts = cleaned.split(/\s+/).map((part) => Number(part));
  if (!parts.length || parts.length > 3) return null;
  if (parts.some((part) => !Number.isFinite(part))) return null;

  const [degrees, minutes = 0, seconds = 0] = parts;
  if (minutes < 0 || minutes >= 60 || seconds < 0 || seconds >= 60) return null;
  let value = Math.abs(degrees) + minutes / 60 + seconds / 3600;
  if (hemisphere === 'S' || hemisphere === 'W' || degrees < 0) value = -value;
  if (!Number.isFinite(value) || Math.abs(value) > limit) return null;
  return value;
}

export function parseLatitude(raw: string): number | null {
  return parseCoordinate(raw, 90);
}

export function parseLongitude(raw: string): number | null {
  return parseCoordinate(raw, 180);
}

export function parseAltitude(raw: string): number | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  const value = Number(trimmed.replace(',', '.'));
  return Number.isFinite(value) ? value : null;
}

/**
 * Project a coordinate onto the offline placeholder map. This is a simple
 * equirectangular projection with a small inset so pins never touch the edge.
 * A real map renderer will replace it later.
 */
export function mapPosition(
  latitude: number | null | undefined,
  longitude: number | null | undefined,
): MapPosition {
  if (!finite(latitude) || !finite(longitude)) {
    return { left: 50, top: 50 };
  }
  const x = (Math.max(-180, Math.min(180, longitude)) + 180) / 360;
  const y = (90 - Math.max(-90, Math.min(90, latitude))) / 180;
  return {
    left: 6 + x * 88,
    top: 8 + y * 84,
  };
}

/** True when both coordinates are present and usable for an external map. */
export function hasCoordinates(
  latitude: number | null | undefined,
  longitude: number | null | undefined,
): boolean {
  return finite(latitude) && finite(longitude);
}
