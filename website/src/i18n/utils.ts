import { defaultLocale, isValidLocale, type Locale } from './config';

type NestedRecord = { [key: string]: string | NestedRecord };

const translationCache = new Map<string, NestedRecord>();

function loadTranslation(locale: Locale): NestedRecord {
  const cached = translationCache.get(locale);
  if (cached) return cached;

  const modules = import.meta.glob('./locales/*.json', { eager: true });
  const key = `./locales/${locale}.json`;
  const mod = modules[key] as { default: NestedRecord } | undefined;
  const data = mod?.default ?? {};
  translationCache.set(locale, data);
  return data;
}

function getNestedValue(obj: NestedRecord, path: string): string | undefined {
  const parts = path.split('.');

  function resolve(current: NestedRecord, idx: number): string | undefined {
    if (idx >= parts.length) return undefined;
    for (let len = parts.length - idx; len >= 1; len--) {
      const key = parts.slice(idx, idx + len).join('.');
      if (!Object.prototype.hasOwnProperty.call(current, key)) continue;
      const next = (current as NestedRecord)[key];
      if (idx + len === parts.length) {
        if (typeof next === 'string') return next;
        continue;
      }
      if (typeof next === 'object' && next !== null && !Array.isArray(next)) {
        const result = resolve(next as NestedRecord, idx + len);
        if (result !== undefined) return result;
      }
    }
    return undefined;
  }

  return resolve(obj, 0);
}

export function useTranslations(locale: Locale) {
  const translations = loadTranslation(locale);
  const fallback = locale !== defaultLocale ? loadTranslation(defaultLocale) : translations;

  return function t(key: string, replacements?: Record<string, string>): string {
    let value = getNestedValue(translations, key) ?? getNestedValue(fallback, key) ?? key;

    if (replacements) {
      for (const [placeholder, replacement] of Object.entries(replacements)) {
        value = value.replace(new RegExp(`\\{${placeholder}\\}`, 'g'), replacement);
      }
    }

    return value;
  };
}

export function getLocaleFromUrl(url: URL): Locale {
  const segments = url.pathname.split('/').filter(Boolean);
  const first = segments[0];
  if (first && isValidLocale(first)) return first;
  return defaultLocale;
}

export function getLocalizedPath(path: string, locale: Locale): string {
  const cleanPath = path.startsWith('/') ? path : `/${path}`;
  if (locale === defaultLocale) return cleanPath;
  return `/${locale}${cleanPath}`;
}

export function removeLocalePrefix(path: string): string {
  const segments = path.split('/').filter(Boolean);
  if (segments[0] && isValidLocale(segments[0])) {
    segments.shift();
  }
  return '/' + segments.join('/');
}
