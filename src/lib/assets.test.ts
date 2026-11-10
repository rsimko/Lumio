import { describe, expect, it } from 'vitest';

import {
  assetFilePaths,
  collectAssetFilePaths,
  sharedAssociationPaths,
} from '@/lib/assets';
import type { PhotoAsset } from '@/types';

function asset(overrides: Partial<PhotoAsset> & Pick<PhotoAsset, 'id' | 'primaryPath'>): PhotoAsset {
  return {
    fileName: 'IMG_0001.CR2',
    stem: 'IMG_0001',
    extension: 'cr2',
    kind: 'raw',
    companionJpeg: null,
    xmpSidecar: null,
    extraSidecars: [],
    extraCompanions: [],
    previewHint: 'embeddedRaw',
    fileSize: 1,
    modifiedMs: 0,
    width: null,
    height: null,
    relativeDir: '',
    ...overrides,
  };
}

describe('asset file helpers', () => {
  it('expands and deduplicates every associated file', () => {
    const photo = asset({
      id: 'a',
      primaryPath: '/photos/IMG_0001.CR2',
      companionJpeg: '/photos/IMG_0001.jpg',
      xmpSidecar: '/photos/IMG_0001.CR2.xmp',
      extraCompanions: ['/photos/IMG_0001.jpeg'],
      extraSidecars: ['/photos/IMG_0001.CR2.xmp', '/photos/IMG_0001.xmp'],
    });
    expect(assetFilePaths(photo)).toEqual([
      '/photos/IMG_0001.CR2',
      '/photos/IMG_0001.jpg',
      '/photos/IMG_0001.jpeg',
      '/photos/IMG_0001.CR2.xmp',
      '/photos/IMG_0001.xmp',
    ]);
    expect(collectAssetFilePaths([photo, photo])).toHaveLength(5);
  });

  it('reports associations shared with photos outside the batch', () => {
    const first = asset({
      id: 'a',
      primaryPath: '/photos/IMG_0001.CR2',
      xmpSidecar: '/photos/shared.xmp',
    });
    const second = asset({
      id: 'b',
      primaryPath: '/photos/IMG_0002.CR2',
      xmpSidecar: '/photos/shared.xmp',
    });
    const unrelated = asset({ id: 'c', primaryPath: '/photos/IMG_0003.CR2' });

    expect(sharedAssociationPaths([first, second, unrelated], [first])).toEqual([
      '/photos/shared.xmp',
    ]);
    expect(sharedAssociationPaths([first, second], [first, second])).toEqual([]);
  });
});
