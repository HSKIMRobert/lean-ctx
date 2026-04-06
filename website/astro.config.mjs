// @ts-check
import { defineConfig } from 'astro/config';
import sitemap from '@astrojs/sitemap';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  site: 'https://leanctx.com',
  i18n: {
    defaultLocale: 'en',
    locales: ['en', 'de', 'ar'],
    routing: {
      prefixDefaultLocale: false,
    },
  },
  integrations: [
    sitemap({
      filter: (page) => !page.includes('/index-backup/'),
      i18n: {
        defaultLocale: 'en',
        locales: {
          en: 'en',
          de: 'de',
          ar: 'ar',
        },
      },
    }),
  ],
  vite: {
    plugins: [tailwindcss()]
  }
});
