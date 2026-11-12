import {
  AppWindow,
  HardDrive,
  Library,
  Palette,
  Settings2,
  type LucideIcon,
} from 'lucide-vue-next';
import type { Component } from 'vue';

import AboutSettings from '@/components/settings/AboutSettings.vue';
import ExternalEditorsSettings from '@/components/settings/ExternalEditorsSettings.vue';
import HiddenPhotosSettings from '@/components/settings/HiddenPhotosSettings.vue';
import IngestSettings from '@/components/settings/IngestSettings.vue';
import LibrarySettings from '@/components/settings/LibrarySettings.vue';
import PreviewCacheSettings from '@/components/settings/PreviewCacheSettings.vue';
import ThemeSettings from '@/components/settings/ThemeSettings.vue';
import ThumbnailSettings from '@/components/settings/ThumbnailSettings.vue';
import TypographySettings from '@/components/settings/TypographySettings.vue';

export interface SettingsTab {
  id: string;
  label: string;
  title: string;
  description: string;
  icon: LucideIcon;
  /** Move a component between these arrays to reshuffle the dialog. */
  sections: Component[];
}

export const SETTINGS_TABS: SettingsTab[] = [
  {
    id: 'appearance',
    label: 'Appearance',
    title: 'Appearance',
    description: 'Theme, typography and how thumbnails are drawn.',
    icon: Palette,
    sections: [ThemeSettings, TypographySettings, ThumbnailSettings],
  },
  {
    id: 'library',
    label: 'Library',
    title: 'Library',
    description: 'Browsing defaults and photos hidden from the library.',
    icon: Library,
    sections: [LibrarySettings, HiddenPhotosSettings],
  },
  {
    id: 'editors',
    label: 'External editors',
    title: 'External editors',
    description: 'Applications Lumio can hand photos to.',
    icon: AppWindow,
    sections: [ExternalEditorsSettings],
  },
  {
    id: 'ingest',
    label: 'Ingest',
    title: 'Ingest',
    description: 'Defaults for copying camera cards into your library.',
    icon: HardDrive,
    sections: [IngestSettings],
  },
  {
    id: 'system',
    label: 'Storage & About',
    title: 'Storage & About',
    description: 'Preview cache information and application details.',
    icon: Settings2,
    sections: [PreviewCacheSettings, AboutSettings],
  },
];
