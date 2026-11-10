<script setup lang="ts">
import { useId } from 'vue';

const gradientId = `lumio-mark-${useId()}`;
</script>

<template>
  <svg
    viewBox="0 0 1024 1024"
    role="img"
    aria-label="Lumio"
    xmlns="http://www.w3.org/2000/svg"
  >
    <defs>
      <linearGradient :id="gradientId" x1="160" y1="96" x2="864" y2="944" gradientUnits="userSpaceOnUse">
        <stop offset="0" stop-color="#4B818A" />
        <stop offset="0.55" stop-color="#2F5C64" />
        <stop offset="1" stop-color="#1B3A41" />
      </linearGradient>
    </defs>
    <rect x="80" y="80" width="864" height="864" rx="196" :fill="`url(#${gradientId})`" />
    <rect
      x="81"
      y="81"
      width="862"
      height="862"
      rx="195"
      fill="none"
      stroke="#FFFFFF"
      stroke-opacity="0.12"
      stroke-width="2"
    />
    <rect x="336" y="272" width="118" height="474" rx="32" fill="#F7F3E8" />
    <rect x="336" y="628" width="364" height="118" rx="32" fill="#F7F3E8" />
    <circle cx="724" cy="312" r="42" fill="#EBC57C" />
  </svg>
</template>
