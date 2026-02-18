#!/usr/bin/env python3
"""Scrape era name (年號) → AD year mappings from Chinese Wikipedia.

Source: https://zh.wikipedia.org/wiki/中国年号列表
Fetched via the MediaWiki Action API with variant=zh-hant for traditional Chinese.

Outputs: scripts/era_years.json
"""

import json
import re
import sys
from pathlib import Path

try:
    import requests
    from bs4 import BeautifulSoup
except ImportError:
    print("Install dependencies: pip install requests beautifulsoup4", file=sys.stderr)
    sys.exit(1)

API_URL = "https://zh.wikipedia.org/w/api.php"
PAGE_TITLE = "中国年号列表"
USER_AGENT = "NaiveTextBot/1.0 (historical text analysis; contact: github)"

# Mapping from Wikipedia section headings to our regime names (traditional Chinese).
# Some headings use simplified Chinese or different phrasing than our regime.rs.
HEADING_TO_REGIME = {
    # Main dynasties
    "西晉": "西晉",
    "西晋": "西晉",
    "東晉": "東晉",
    "东晋": "東晉",
    "南朝宋": "劉宋",
    "劉宋": "劉宋",
    "刘宋": "劉宋",
    "南朝齊": "南齊",
    "南朝齐": "南齊",
    "南齊": "南齊",
    "南齐": "南齊",
    "南朝梁": "梁",
    "梁": "梁",
    "南朝陳": "陳",
    "南朝陈": "陳",
    "陳": "陳",
    "陈": "陳",
    "北魏": "北魏",
    # Sixteen Kingdoms
    "漢趙": "漢趙",
    "汉赵": "漢趙",
    "前趙": "漢趙",
    "前赵": "漢趙",
    "後趙": "後趙",
    "后赵": "後趙",
    "成漢": "成漢",
    "成汉": "成漢",
    "前涼": "前涼",
    "前凉": "前涼",
    "前燕": "前燕",
    "前秦": "前秦",
    "後秦": "後秦",
    "后秦": "後秦",
    "後燕": "後燕",
    "后燕": "後燕",
    "西秦": "西秦",
    "後涼": "後涼",
    "后凉": "後涼",
    "南涼": "南涼",
    "南凉": "南涼",
    "南燕": "南燕",
    "西涼": "西涼",
    "西凉": "西涼",
    "北涼": "北涼",
    "北凉": "北涼",
    "夏": "夏",
    "胡夏": "夏",
    "赫連夏": "夏",
    "北燕": "北燕",
    # Single-character matches (careful — only match when heading is exactly this)
    "宋": "劉宋",
    "齊": "南齊",
    "齐": "南齊",
}

# All regimes we care about
TARGET_REGIMES = set(HEADING_TO_REGIME.values())

# Headings that indicate we've left our target regimes. When we see these,
# reset current_regime to None to avoid misattributing subsequent tables.
STOP_HEADINGS = {
    "東魏", "东魏", "西魏", "北齊", "北齐", "北周",
    "隋朝", "隋", "唐朝", "唐",
    "參見", "参见", "參考文獻", "参考文献", "注釋", "注释", "外部連結", "外部链接",
}


def fetch_wikipedia_html() -> str:
    """Fetch the parsed HTML of the Chinese era names list from Wikipedia."""
    params = {
        "action": "parse",
        "page": PAGE_TITLE,
        "prop": "text",
        "format": "json",
        "formatversion": "2",
        "variant": "zh-hant",
    }
    headers = {"User-Agent": USER_AGENT}
    print(f"Fetching {API_URL}?page={PAGE_TITLE} ...", file=sys.stderr)
    resp = requests.get(API_URL, params=params, headers=headers, timeout=30)
    resp.raise_for_status()
    data = resp.json()
    return data["parse"]["text"]


def extract_year_from_text(text: str) -> int | None:
    """Extract an AD year number from text like '265年' or '265年十二月'."""
    # Match patterns like "265年", "前140年" (BC dates, skip those)
    m = re.search(r"(?:前)?(\d+)年", text)
    if m:
        year = int(m.group(1))
        if "前" in text[:m.start() + 2]:
            return None  # BC date, not relevant
        return year
    return None


def parse_year_range(text: str) -> tuple[int | None, int | None]:
    """Parse a year range like '265年－274年' or '290年' into (start, end)."""
    text = text.strip()
    # Try to find a range with separator (－, —, –, -, ~, 至)
    for sep in ["－", "—", "–", "-", "～", "~", "至"]:
        if sep in text:
            parts = text.split(sep, 1)
            start = extract_year_from_text(parts[0])
            end = extract_year_from_text(parts[1])
            if start and end:
                return start, end
            if start:
                return start, start
    # Single year
    year = extract_year_from_text(text)
    if year:
        return year, year
    return None, None


def clean_heading(text: str) -> str:
    """Clean a heading: strip edit links, year ranges, whitespace."""
    text = text.strip()
    text = re.sub(r"\[.*?\]", "", text).strip()
    text = re.sub(r"[（(][^)）]*[)）]", "", text).strip()
    text = text.rstrip("：:。. ")
    return text


def normalize_heading(text: str) -> str | None:
    """Extract the regime name from a heading like '西晉（266年—316年）'.

    Returns the regime name if matched, or None if unrecognized.
    """
    text = clean_heading(text)
    # Direct lookup
    if text in HEADING_TO_REGIME:
        return HEADING_TO_REGIME[text]
    # Partial match — heading might contain extra text
    for key, regime in HEADING_TO_REGIME.items():
        if len(key) >= 2 and key in text:
            return regime
    return None


def is_stop_heading(text: str) -> bool:
    """Check if a heading indicates we've left the target period."""
    text = clean_heading(text)
    for stop in STOP_HEADINGS:
        if stop in text:
            return True
    return False


def parse_tables(html: str) -> list[dict]:
    """Parse era name tables from the Wikipedia HTML."""
    soup = BeautifulSoup(html, "html.parser")
    results = []

    # Strategy: walk through all headings (h2, h3, h4) and find the nearest
    # following table. Each heading identifies a regime.
    current_regime = None

    for element in soup.find_all(["h2", "h3", "h4", "table"]):
        if element.name in ("h2", "h3", "h4"):
            heading_text = element.get_text(strip=True)
            heading_text = re.sub(r"\[.*?\]", "", heading_text).strip()

            if is_stop_heading(heading_text):
                current_regime = None
                continue

            regime = normalize_heading(heading_text)
            if regime:
                current_regime = regime
            elif element.name in ("h2", "h3"):
                # Major section heading that we don't recognize as a target
                # regime — reset to avoid misattributing following tables.
                # (Skip h4 since they might be sub-sections within a regime.)
                current_regime = None

        elif element.name == "table" and current_regime:
            # Parse the table rows
            rows = element.find_all("tr")
            for row in rows:
                cells = row.find_all(["td", "th"])
                if len(cells) < 2:
                    continue

                # First cell is usually the era name, second is the year range
                era_text = cells[0].get_text(strip=True)
                year_text = cells[1].get_text(strip=True)

                # Skip header rows
                if era_text in ("年號", "年号", "紀年", "纪年", ""):
                    continue
                # Skip rows that are clearly not era names
                if len(era_text) > 6 or len(era_text) < 2:
                    continue

                start_ad, end_ad = parse_year_range(year_text)
                if start_ad is None:
                    continue
                if end_ad is None:
                    end_ad = start_ad

                # Sanity check: should be in range 200-600 AD
                if not (200 <= start_ad <= 600 and 200 <= end_ad <= 600):
                    continue

                results.append({
                    "regime": current_regime,
                    "era": era_text,
                    "start_ad": start_ad,
                    "end_ad": end_ad,
                })

    return results


# Known character variants between regime.rs and Wikipedia.
# regime.rs name → Wikipedia name
ERA_CHAR_VARIANTS = {
    ("北魏", "神麚"): "神䴥",   # 麚 → 䴥 (rare CJK variant)
    ("成漢", "太武"): "建初",   # 太武 is Li Te's era; Wikipedia lists as 建初
    ("前涼", "太初"): None,     # 前涼 didn't use 太初; used 西晉 eras. Manual: 314-320
}

# Manual fallback AD years for entries that Wikipedia doesn't list.
MANUAL_FALLBACKS = {
    ("前涼", "太初"): (314, 320),
}


def read_regime_rs() -> list[dict]:
    """Read existing ERA_NAMES from src/regime.rs for cross-validation."""
    rs_path = Path(__file__).parent.parent / "src" / "regime.rs"
    if not rs_path.exists():
        print(f"Warning: {rs_path} not found, skipping cross-validation", file=sys.stderr)
        return []

    content = rs_path.read_text(encoding="utf-8")

    # Map Regime enum variant to Chinese name
    variant_to_chinese = {}
    for m in re.finditer(r'Self::(\w+)\s*=>\s*"([^"]+)"', content):
        variant_to_chinese[m.group(1)] = m.group(2)

    # Extract all EraEntry entries
    entries = []
    for m in re.finditer(
        r'EraEntry\s*\{\s*name:\s*"([^"]+)"\s*,\s*regime:\s*Regime::(\w+)',
        content,
    ):
        era_name = m.group(1)
        variant = m.group(2)
        chinese = variant_to_chinese.get(variant, variant)
        entries.append({"regime": chinese, "era": era_name})

    return entries


def build_matched_output(scraped: list[dict], existing: list[dict]) -> list[dict]:
    """Match each regime.rs entry to a scraped entry, returning the final output.

    Handles character variants and manual fallbacks.
    """
    # Build lookup: (regime, era) → scraped entry (first match wins)
    scraped_lookup: dict[tuple[str, str], dict] = {}
    for entry in scraped:
        key = (entry["regime"], entry["era"])
        if key not in scraped_lookup:
            scraped_lookup[key] = entry

    matched = []
    missing = []

    for entry in existing:
        regime, era = entry["regime"], entry["era"]
        key = (regime, era)

        # Direct match
        if key in scraped_lookup:
            s = scraped_lookup[key]
            matched.append({
                "regime": regime,
                "era": era,
                "start_ad": s["start_ad"],
                "end_ad": s["end_ad"],
            })
            continue

        # Try character variant
        variant_era = ERA_CHAR_VARIANTS.get(key)
        if variant_era:
            variant_key = (regime, variant_era)
            if variant_key in scraped_lookup:
                s = scraped_lookup[variant_key]
                matched.append({
                    "regime": regime,
                    "era": era,
                    "start_ad": s["start_ad"],
                    "end_ad": s["end_ad"],
                })
                print(f"  Variant match: {regime}/{era} → {variant_era} ({s['start_ad']}-{s['end_ad']})", file=sys.stderr)
                continue

        # Manual fallback
        if key in MANUAL_FALLBACKS:
            start, end = MANUAL_FALLBACKS[key]
            matched.append({
                "regime": regime,
                "era": era,
                "start_ad": start,
                "end_ad": end,
            })
            print(f"  Manual fallback: {regime}/{era} ({start}-{end})", file=sys.stderr)
            continue

        missing.append(key)

    if missing:
        print(f"\nWARNING: {len(missing)} entries in regime.rs could NOT be matched:", file=sys.stderr)
        for regime, era in missing:
            print(f"  {regime}/{era}", file=sys.stderr)

    # Also report extra scraped entries (informational)
    existing_set = {(e["regime"], e["era"]) for e in existing}
    variant_targets = {v for v in ERA_CHAR_VARIANTS.values() if v}
    extra = [
        e for e in scraped
        if (e["regime"], e["era"]) not in existing_set
        and e["era"] not in variant_targets
    ]
    if extra:
        print(f"\nINFO: {len(extra)} extra entries in Wikipedia not in regime.rs (rebels, pretenders, etc.)", file=sys.stderr)

    return matched


def main():
    # Fetch and parse
    html = fetch_wikipedia_html()
    scraped = parse_tables(html)

    print(f"Scraped {len(scraped)} era entries from Wikipedia", file=sys.stderr)

    # Count by regime
    by_regime: dict[str, int] = {}
    for entry in scraped:
        by_regime[entry["regime"]] = by_regime.get(entry["regime"], 0) + 1
    for regime in sorted(by_regime.keys()):
        print(f"  {regime}: {by_regime[regime]} eras", file=sys.stderr)

    # Match against regime.rs entries
    existing = read_regime_rs()
    if existing:
        print(f"\nMatching against {len(existing)} regime.rs entries...", file=sys.stderr)
        matched = build_matched_output(scraped, existing)
    else:
        print("No regime.rs found, outputting raw scraped data", file=sys.stderr)
        matched = scraped

    # Write output — only matched entries (aligned with regime.rs)
    out_path = Path(__file__).parent / "era_years.json"
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(matched, f, ensure_ascii=False, indent=2)
    print(f"\nWrote {out_path} ({len(matched)} entries, matched to regime.rs)", file=sys.stderr)


if __name__ == "__main__":
    main()
