import re
from pathlib import Path

import markdown
from bs4 import BeautifulSoup, NavigableString

import report_charts as charts

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "docs" / "BUSINESS_PLAN.md"
TARGET = ROOT / "docs" / "BUSINESS_PLAN.html"
STYLE = ROOT / "scripts" / "report_style.css"
SCRIPT = ROOT / "scripts" / "report_app.js"


def convert_markdown(text):
    text = re.sub(r"^(\s*)- \[ \] ", r"\1- ☐ ", text, flags=re.MULTILINE)
    text = re.sub(r"^(\s*)- \[x\] ", r"\1- ☑ ", text, flags=re.MULTILINE)
    return markdown.markdown(
        text,
        extensions=["tables", "fenced_code", "sane_lists"],
        output_format="html5",
    )


def add_ids(soup):
    for index, heading in enumerate(soup.find_all(["h2", "h3"]), start=1):
        heading["id"] = f"sec-{index}"
    first = soup.find("h2")
    if first:
        first["id"] = "summary"


def build_toc(soup):
    items = []
    for heading in soup.find_all(["h2", "h3"]):
        level = int(heading.name[1])
        text = heading.get_text(" ", strip=True)
        items.append((level, heading.get("id", ""), text))
    return items


def toc_markup(items):
    links = []
    for level, anchor, text in items:
        links.append(
            f'<a class="toc-link lvl{level}" href="#{anchor}">{text}</a>'
        )
    return "".join(links)


def insert_after(soup, prefix, html, remove_following_pre=False):
    target = None
    for heading in soup.find_all(["h2", "h3", "h4"]):
        if heading.get_text(" ", strip=True).startswith(prefix):
            target = heading
            break
    if target is None:
        return False
    parsed = BeautifulSoup(html, "html.parser")
    for node in reversed(list(parsed.contents)):
        target.insert_after(node)
    if remove_following_pre:
        pre = target.find_next_sibling("pre")
        if pre is not None:
            pre.decompose()
    return True


def wrap_tables(soup):
    for table in soup.find_all("table"):
        classes = table.get("class", [])
        table["class"] = classes + ["data-table"]
        wrapper = soup.new_tag("div")
        wrapper["class"] = "table-wrap"
        table.wrap(wrapper)


def external_links(soup):
    for link in soup.find_all("a", href=True):
        href = link["href"]
        if href.startswith("http://") or href.startswith("https://"):
            link["target"] = "_blank"
            link["rel"] = "noopener noreferrer"


def add_anchors(soup):
    for heading in soup.find_all(["h2", "h3"]):
        anchor = soup.new_tag("a", href=f"#{heading.get('id', '')}")
        anchor["class"] = "anchor"
        anchor["aria-label"] = "Ссылка на раздел"
        anchor.string = "#"
        heading.append(anchor)


def clean_intro(soup):
    first_h2 = soup.find("h2")
    if first_h2 is None:
        return
    for child in list(soup.children):
        if getattr(child, "name", None) == "h2":
            break
        if isinstance(child, NavigableString):
            continue
        if child.name in ("h1", "p"):
            child.decompose()


def reading_minutes(soup):
    text = soup.get_text(" ", strip=True)
    words = re.findall(r"[A-Za-zА-Яа-яЁё0-9]+", text)
    return max(1, round(len(words) / 180))


def hero(minutes, section_count):
    return f"""
<header class="hero">
  <div class="hero-top">
    <div>
      <div class="eyebrow">QuietReader · стратегия монетизации</div>
      <h1>QuietReader<span>бизнес-план монетизации</span></h1>
      <p class="lede">Как превратить сильную читалку в устойчивый продукт: рынок, аудитория, цена, каналы, B2B и дизайн.</p>
      <div class="meta">
        <span>Версия 1.0</span>
        <span>22.10.2026</span>
        <span>{section_count} разделов</span>
        <span>~{minutes} мин чтения</span>
      </div>
      <div class="hero-actions">
        <a class="btn primary" href="#summary">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 5.5A2.5 2.5 0 0 1 6.5 3H20v18H6.5A2.5 2.5 0 0 1 4 18.5z"/><path d="M8 7h8M8 11h6"/></svg>
          К резюме
        </a>
        <button class="btn" id="themeButton" type="button" title="Сменить тему" aria-label="Тема: Светлая">
          <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></svg>
          <span id="themeLabel">Светлая</span>
        </button>
        <button class="btn" id="printButton" type="button">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9V3h12v6M6 18H4a1 1 0 0 1-1-1v-6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v6a1 1 0 0 1-1 1h-2"/><rect x="6" y="14" width="12" height="7" rx="1"/></svg>
          Печать
        </button>
        <a class="btn" href="BUSINESS_PLAN.md">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v12M7 10l5 5 5-5M5 21h14"/></svg>
          Markdown
        </a>
      </div>
    </div>
    <div class="hero-quote">
      <p>Продукт сильный. Канал — нулевой. 90 дней — на упаковку и дистрибуцию.</p>
      <span>Главный вывод</span>
    </div>
  </div>
  <div class="kpi-grid">
    <div class="kpi"><b>$28 млн</b><span>TAM в год · оценка</span></div>
    <div class="kpi"><b>$8–15 млн</b><span>SAM · доступный сегмент</span></div>
    <div class="kpi"><b>$15–18 тыс.</b><span>выручка · год 1</span></div>
    <div class="kpi"><b>$115 тыс.</b><span>выручка · год 3</span></div>
    <div class="kpi"><b>$29.99</b><span>Pro Lifetime</span></div>
    <div class="kpi"><b>200–250</b><span>продаж до безубыточности</span></div>
  </div>
</header>
"""


def bets():
    return """
<div class="bets">
  <div class="bet"><span class="tag">Ставка 1</span><h3>B2C Pro</h3><p>Быстрый вход и низкий CAC: RU/CIS, self-hosted, манга и доступность.</p><b>$29.99</b></div>
  <div class="bet"><span class="tag">Ставка 2</span><h3>B2B и образование</h3><p>Высокий чек и предсказуемость, но медленный цикл продаж.</p><b>$299–1 499 / год</b></div>
  <div class="bet"><span class="tag">Ставка 3</span><h3>Sync</h3><p>Подписка только на то, что реально стоит денег: синхронизация и инфраструктура.</p><b>$39 / год</b></div>
</div>
<div class="callout">
  <div class="callout-icon">i</div>
  <div>
    <h4>Как читать этот отчёт</h4>
    <p>Если у вас 10 минут — разделы 0, 5.7–5.8, 7 и 10. Если 30 минут — добавьте 2, 8, 9, 12 и 14. Полный документ — для решений по цене, каналам и B2B.</p>
  </div>
</div>
"""


def compose(soup, toc_items, minutes, section_count):
    content = str(soup)
    side_links = toc_markup(toc_items)
    mobile_links = toc_markup(toc_items)
    style = STYLE.read_text(encoding="utf-8")
    script = SCRIPT.read_text(encoding="utf-8")
    return f"""<!DOCTYPE html>
<html lang="ru" data-theme="light">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="description" content="QuietReader — бизнес-план монетизации: рынок, аудитория, цена, каналы, B2B, дизайн и финансовая модель.">
  <title>QuietReader — бизнес-план монетизации</title>
  <style>{style}</style>
</head>
<body>
  <div class="progress" aria-hidden="true"><span id="progressValue"></span></div>
  {hero(minutes, section_count)}
  <div class="layout">
    <aside class="sidebar">
      <div class="side-title"><b>Содержание</b><span>{section_count} разделов</span></div>
      <input class="toc-search" id="tocSearch" type="search" placeholder="Найти раздел…" aria-label="Поиск по содержанию">
      <nav class="toc" aria-label="Содержание отчёта">{side_links}</nav>
      <div class="side-foot">
        QuietReader · v0.7.1
        <a href="BUSINESS_PLAN.md">Исходный Markdown</a>
        <a href="#summary">Наверх</a>
      </div>
    </aside>
    <main class="content" id="content">
      <details class="mobile-toc">
        <summary>Содержание · {len(toc_items)} пунктов</summary>
        <nav class="toc" aria-label="Содержание отчёта">{mobile_links}</nav>
      </details>
      {bets()}
      {content}
    </main>
  </div>
  <button class="menu-btn" id="menuButton" type="button" aria-label="Открыть содержание"><i></i>Содержание</button>
  <button class="to-top" id="toTop" type="button" aria-label="Наверх">
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 19V5M6 11l6-6 6 6"/></svg>
  </button>
  <script>{script}</script>
</body>
</html>
"""


def main():
    markdown_text = SOURCE.read_text(encoding="utf-8")
    html = convert_markdown(markdown_text)
    soup = BeautifulSoup(html, "html.parser")
    clean_intro(soup)
    add_ids(soup)
    toc_items = build_toc(soup)
    section_count = len(soup.find_all("h2"))
    minutes = reading_minutes(soup)
    wrap_tables(soup)
    external_links(soup)

    insert_after(soup, "2.2. TAM / SAM / SOM", charts.market_ladder())
    insert_after(soup, "3.6. Позиционная карта", charts.quadrant(), remove_following_pre=True)
    insert_after(soup, "Какой процент аудитории действительно", charts.design_sensitivity())
    insert_after(soup, "5.8. Оценка текущих макетов", charts.design_block())
    insert_after(soup, "7.1. Прайс-лист", charts.pricing_cards())
    insert_after(soup, "7.3. Юнит-экономика", charts.waterfall())
    insert_after(soup, "8.1. Каналы", charts.channel_heatmap())
    insert_after(soup, "9.1. Воронка", charts.funnel_chart())
    insert_after(soup, "10.1. Сценарии", charts.revenue_scenarios())
    insert_after(soup, "10.4. Чувствительность", charts.sensitivity())
    insert_after(soup, "12. Риски", '<h3 class="sr-only">Инфографика: риски</h3>' + charts.risk_matrix())
    insert_after(soup, "14. Дорожная карта", '<h3 class="sr-only">Инфографика: план</h3>' + charts.roadmap())
    insert_after(soup, "15. Метрики", '<h3 class="sr-only">Инфографика: метрики</h3>' + charts.metrics())

    add_anchors(soup)
    TARGET.write_text(compose(soup, toc_items, minutes, section_count), encoding="utf-8")
    print(f"Wrote {TARGET}")
    print(f"Sections: {len(toc_items)}, reading time: {minutes} min, size: {TARGET.stat().st_size} bytes")


if __name__ == "__main__":
    main()
