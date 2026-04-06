export const defaultLocale = 'en' as const;

export const locales = ['en', 'de', 'ar'] as const;

export type Locale = (typeof locales)[number];

export const rtlLocales: readonly Locale[] = ['ar'];

export const localeNames: Record<Locale, string> = {
  en: 'English',
  de: 'Deutsch',
  ar: 'العربية',
};

export const localeFlags: Record<Locale, string> = {
  en: '🇬🇧',
  de: '🇩🇪',
  ar: '🇸🇦',
};

export function isRtl(locale: Locale): boolean {
  return rtlLocales.includes(locale);
}

export function isValidLocale(value: string): value is Locale {
  return locales.includes(value as Locale);
}
