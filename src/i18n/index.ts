import en, { type Key } from "./locales/en";
import ru from "./locales/ru";
import uk from "./locales/uk";
import fr from "./locales/fr";
import es from "./locales/es";
import ja from "./locales/ja";
import zh from "./locales/zh";
import ko from "./locales/ko";
import tr from "./locales/tr";
import it from "./locales/it";
import pl from "./locales/pl";
import cs from "./locales/cs";

export type { Key };

export const languages = [
  { code: "en", name: "English", bcp: "en-US" },
  { code: "ru", name: "Русский", bcp: "ru-RU" },
  { code: "uk", name: "Українська", bcp: "uk-UA" },
  { code: "fr", name: "Français", bcp: "fr-FR" },
  { code: "es", name: "Español", bcp: "es-ES" },
  { code: "ja", name: "日本語", bcp: "ja-JP" },
  { code: "zh", name: "简体中文", bcp: "zh-CN" },
  { code: "ko", name: "한국어", bcp: "ko-KR" },
  { code: "tr", name: "Türkçe", bcp: "tr-TR" },
  { code: "it", name: "Italiano", bcp: "it-IT" },
  { code: "pl", name: "Polski", bcp: "pl-PL" },
  { code: "cs", name: "Čeština", bcp: "cs-CZ" },
] as const;

export type Lang = (typeof languages)[number]["code"];

const dictionaries: Record<Lang, Record<Key, string>> = { en, ru, uk, fr, es, ja, zh, ko, tr, it, pl, cs };

export type Translate = (key: Key, vars?: Record<string, string | number>) => string;

export function makeTranslate(lang: Lang): Translate {
  const dict = dictionaries[lang] ?? en;
  return (key, vars) => {
    const template = dict[key] ?? en[key] ?? key;
    return vars ? template.replace(/\{(\w+)\}/g, (_, k) => String(vars[k] ?? "")) : template;
  };
}

export const bcpOf = (lang: Lang) => languages.find((l) => l.code === lang)?.bcp ?? "en-US";
