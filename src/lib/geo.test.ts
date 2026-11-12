import { describe, expect, it } from 'vitest';

import {
  formatAltitude,
  formatCoordinates,
  formatLatitude,
  formatLongitude,
  hasCoordinates,
  mapPosition,
  parseAltitude,
  parseLatitude,
  parseLongitude,
} from '@/lib/geo';

describe('coordinate formatting', () => {
  it('adds hemispheres and keeps five decimal places', () => {
    expect(formatLatitude(48.8566)).toBe('48.85660° N');
    expect(formatLatitude(-33.8688)).toBe('33.86880° S');
    expect(formatLongitude(2.3508)).toBe('2.35080° E');
    expect(formatLongitude(-118.735)).toBe('118.73500° W');
    expect(formatCoordinates(48.8566, -2.3508)).toBe('48.85660° N, 2.35080° W');
    expect(formatCoordinates(null, null)).toBe('');
  });

  it('formats altitude relative to sea level', () => {
    expect(formatAltitude(125)).toBe('125 m above sea level');
    expect(formatAltitude(-12.5)).toBe('12.5 m below sea level');
    expect(formatAltitude(null)).toBe('');
  });
});

describe('coordinate parsing', () => {
  it('accepts decimal degrees and hemispheres', () => {
    expect(parseLatitude('48.8566')).toBeCloseTo(48.8566, 5);
    expect(parseLatitude('48.8566 N')).toBeCloseTo(48.8566, 5);
    expect(parseLatitude('33.8688 S')).toBeCloseTo(-33.8688, 5);
    expect(parseLongitude('118.735 W')).toBeCloseTo(-118.735, 5);
    expect(parseLatitude('-90')).toBe(-90);
  });

  it('accepts simple degrees, minutes and seconds', () => {
    expect(parseLatitude(`48° 51' 24" N`)).toBeCloseTo(48.8567, 3);
    expect(parseLongitude('118 44 6 W')).toBeCloseTo(-118.735, 3);
  });

  it('rejects out-of-range and malformed values', () => {
    expect(parseLatitude('91')).toBeNull();
    expect(parseLongitude('180.1')).toBeNull();
    expect(parseLatitude('48 75 0 N')).toBeNull();
    expect(parseLatitude('north')).toBeNull();
    expect(parseLatitude('')).toBeNull();
  });

  it('parses altitude values', () => {
    expect(parseAltitude('125')).toBe(125);
    expect(parseAltitude('-12.5')).toBe(-12.5);
    expect(parseAltitude('')).toBeNull();
    expect(parseAltitude('high')).toBeNull();
  });
});

describe('placeholder map projection', () => {
  it('centres a missing location', () => {
    expect(mapPosition(null, null)).toEqual({ left: 50, top: 50 });
    expect(hasCoordinates(null, 2)).toBe(false);
    expect(hasCoordinates(2, 4)).toBe(true);
  });

  it('projects coordinates into the inset map area', () => {
    expect(mapPosition(0, 0)).toEqual({ left: 50, top: 50 });
    expect(mapPosition(90, -180)).toEqual({ left: 6, top: 8 });
    expect(mapPosition(-90, 180)).toEqual({ left: 94, top: 92 });
  });
});
