import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { locales } from "./i18n/config";
import { buildPageMetadata, IDENTITY_PHRASE, OG_ALT, SITE_NAME, SITE_URL } from "./page-meta";

/** hreflang alternates derive from the canonical locale registry. */
function expectedLanguages(path: string): Record<string, string> {
  const languages: Record<string, string> = {};
  for (const l of locales) {
    languages[l] = `${SITE_URL}/${l}${path}`;
  }
  languages["x-default"] = `${SITE_URL}/en${path}`;
  return languages;
}

describe("page metadata", () => {
  it.each([
    ["en", "/faq", "FAQ · Codewhale", "en_US"],
    ["zh", "/faq", "常见问题 · Codewhale", "zh_CN"],
    ["en", "/feed", "Activity · Codewhale", "en_US"],
    ["zh", "/feed", "动态 · Codewhale", "zh_CN"],
    ["en", "/roadmap", "Roadmap · Codewhale", "en_US"],
    ["zh", "/roadmap", "路线图 · Codewhale", "zh_CN"],
    ["ru", "/faq", "FAQ · Codewhale", "ru_RU"],
    ["uk", "/faq", "FAQ · Codewhale", "uk_UA"],
    ["pt-BR", "/install", "Install · Codewhale", "pt_BR"],
  ])("builds canonical, hreflang, Open Graph, and Twitter fields for %s%s", (locale, path, title, ogLocale) => {
    const description = `${locale} metadata contract`;
    const metadata = buildPageMetadata({ path, locale, title, description });
    const canonical = `${SITE_URL}/${locale}${path}`;

    expect(metadata.alternates).toEqual({
      canonical,
      languages: expectedLanguages(path),
    });
    expect(metadata.openGraph).toEqual({
      title,
      description,
      url: canonical,
      siteName: SITE_NAME,
      type: "website",
      locale: ogLocale,
      images: [
        {
          url: `${SITE_URL}/opengraph-image`,
          width: 1200,
          height: 630,
          alt: OG_ALT,
        },
      ],
    });
    expect(metadata.twitter).toEqual({
      card: "summary_large_image",
      title,
      description,
      images: [`${SITE_URL}/opengraph-image`],
    });
  });

  it("emits an hreflang alternate for every routed locale plus x-default", () => {
    const metadata = buildPageMetadata({
      path: "/docs",
      locale: "ja",
      title: "Docs · Codewhale",
      description: "hreflang coverage",
    });
    const languages = metadata.alternates?.languages as Record<string, string>;
    for (const l of locales) {
      expect(languages[l], `missing hreflang for ${l}`).toBe(`${SITE_URL}/${l}/docs`);
    }
    expect(languages["x-default"]).toBe(`${SITE_URL}/en/docs`);
    expect(Object.keys(languages)).toHaveLength(locales.length + 1);
  });

  it("keeps the canonical brand-first identity without duplicating an OG heading", () => {
    const brand = new RegExp(`\\b${SITE_NAME}\\b`, "gi");
    const ogImage = readFileSync(new URL("../app/opengraph-image.tsx", import.meta.url), "utf8");

    expect(IDENTITY_PHRASE).toBe("Codewhale dives into the deep so you don't have to.");
    expect(OG_ALT).toBe(IDENTITY_PHRASE);
    expect(OG_ALT.match(brand)).toHaveLength(1);
    expect(ogImage).toContain("{IDENTITY_PHRASE}");
    expect(ogImage).not.toContain("{SITE_NAME}");
  });

  it("keeps the previously incomplete indexable routes on the shared helper", () => {
    for (const [route, path] of [
      ["faq", "/faq"],
      ["feed", "/feed"],
      ["roadmap", "/roadmap"],
    ]) {
      const source = readFileSync(
        new URL(`../app/[locale]/${route}/page.tsx`, import.meta.url),
        "utf8",
      );
      expect(source, route).toContain('import { buildPageMetadata } from "@/lib/page-meta"');
      expect(source, route).toContain("return buildPageMetadata({");
      expect(source, route).toContain(`path: "${path}"`);
    }
  });
});
