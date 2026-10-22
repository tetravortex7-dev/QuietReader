def chart_card(kicker, title, badge, body, note="", legend=""):
    legend_html = f'<div class="chart-legend">{legend}</div>' if legend else ""
    note_html = f'<div class="chart-note">{note}</div>' if note else ""
    return f"""
<div class="chart-card">
  <div class="chart-head">
    <div>
      <div class="chart-kicker">{kicker}</div>
      <h4>{title}</h4>
    </div>
    <div class="chart-badge">{badge}</div>
  </div>
  {body}
  {legend_html}
  {note_html}
</div>
"""


def market_ladder():
    body = """
<div class="ladder">
  <div class="ladder-step"><span class="ladder-value">1,4 млрд</span><span class="ladder-label">ПК в мире</span></div>
  <div class="ladder-step"><span class="ladder-value">140 млн</span><span class="ladder-label">читают длинные тексты · 10%</span></div>
  <div class="ladder-step"><span class="ladder-value">1,4 млн</span><span class="ladder-label">готовы платить · 1%</span></div>
  <div class="ladder-step hot"><span class="ladder-value">$28 млн</span><span class="ladder-label">TAM в год · при $20</span></div>
  <div class="ladder-step olive"><span class="ladder-value">$8–15 млн</span><span class="ladder-label">SAM · доступный сегмент</span></div>
  <div class="ladder-step"><span class="ladder-value">$40–160 тыс.</span><span class="ladder-label">SOM · 2–8 тыс. покупок за 3 года</span></div>
</div>
"""
    return chart_card(
        "Рынок",
        "Воронка рынка: от 1,4 млрд ПК до $28 млн",
        "Оценка",
        body,
        "TAM — не рынок «всех электронных книг», а узкий сегмент платного софта для чтения на десктопе. $14,5 млрд — это потребительские расходы на контент, их нельзя присваивать приложению.",
    )


def revenue_scenarios():
    data = [
        ("Год 1", [("1,2", "conservative", 0.4), ("15,1", "base", 5.0), ("57,5", "optimistic", 19.2)]),
        ("Год 2", [("7,4", "conservative", 2.5), ("51,5", "base", 17.2), ("150,2", "optimistic", 50.1)]),
        ("Год 3", [("18,0", "conservative", 6.0), ("117,0", "base", 39.0), ("290,5", "optimistic", 96.8)]),
    ]
    groups = []
    for label, bars in data:
        cells = "".join(
            f'<div class="bar {kind}" style="--h:{height}"><span>{value}</span></div>'
            for value, kind, height in bars
        )
        groups.append(f'<div class="bar-group"><div class="bar-stack">{cells}</div><div class="bar-label">{label}</div></div>')
    body = f'<div class="bar-chart">{"".join(groups)}</div>'
    legend = (
        '<span><i class="legend-dot conservative"></i>Консервативный</span>'
        '<span><i class="legend-dot base"></i>Базовый</span>'
        '<span><i class="legend-dot optimistic"></i>Оптимистичный</span>'
    )
    return chart_card(
        "Финансы",
        "Три сценария выручки, $ тыс. в год",
        "Базовый = $117 тыс. к году 3",
        body,
        "Базовый сценарий: 540 покупок в год 1, 1 500 — в год 2, 3 200 — в год 3; ARPU $16–18 с учётом Sync и B2B. Оптимистичный сценарий требует канала и партнёрств, а не только продукта.",
        legend,
    )


def design_sensitivity():
    rows = [
        ("macOS / Setapp", 60),
        ("Массовый читатель", 50),
        ("Манга и комиксы", 50),
        ("Слабовидящие", 45),
        ("Студенты", 40),
        ("Дислексия", 40),
        ("B2B и образование", 30),
        ("Self-hosted", 22),
        ("Power users", 18),
    ]
    body = "".join(
        f'<div class="hbar-row"><span class="hbar-label">{label}</span><div class="hbar-track"><div class="hbar-fill" style="width:{value}%"></div></div><span class="hbar-value">{value}%</span></div>'
        for label, value in rows
    )
    return chart_card(
        "Дизайн",
        "Кто действительно покупает дизайн",
        "Доля сегмента, чувствительная к UI",
        body,
        "Дизайн — не универсальный множитель. Сильнее всего он влияет на macOS/Setapp, массового читателя и мангу; слабее — на power users, которые терпят некрасивый интерфейс ради функций.",
    )


def design_coverage():
    rows = [
        ("Cozy", 85, 88),
        ("Массовый", 55, 80),
        ("Студенты", 45, 75),
        ("macOS", 65, 80),
        ("Манга", 35, 80),
        ("Дислексия", 40, 75),
        ("Power", 30, 70),
        ("B2B", 25, 65),
        ("Self-hosted", 22, 60),
    ]
    items = []
    for label, old, new in rows:
        left = min(old, new)
        width = max(old, new) - left
        items.append(
            f'<div class="dumbbell-row"><span class="dumbbell-label">{label}</span>'
            f'<div class="dumbbell-track"><span class="dumbbell-line" style="left:{left}%;width:{width}%"></span>'
            f'<span class="dumbbell-dot old" style="left:{old}%"></span>'
            f'<span class="dumbbell-dot new" style="left:{new}%"></span></div>'
            f'<span class="dumbbell-value">{old} → {new}%</span></div>'
        )
    body = "".join(items)
    legend = (
        '<span><i class="legend-dot old"></i>Сейчас</span>'
        '<span><i class="legend-dot base"></i>С режимами Cozy / Studio / Document / Comic</span>'
    )
    return chart_card(
        "Дизайн",
        "Покрытие аудитории: сейчас и после режимов",
        "Оценка",
        body,
        "Режимы поднимают покрытие с ~45–50% до ~75–80%, но не превращают всех в покупателей. Реалистичный прирост конверсии — 10–20% при уже существующем трафике.",
        legend,
    )


def design_scores():
    scores = [
        ("9,5", "Экран чтения", "Визуал и атмосфера: бумага, серифы, буквица, сноски."),
        ("7,0", "Ясность чтения", "Не хватает управления шрифтом, полями и фокус-режима."),
        ("9,0", "Библиотека", "Тёплый «Cozy»-экран выглядит дорого."),
        ("5,5", "Ясность библиотеки", "Много декора и вымышленных функций, слабая иерархия."),
    ]
    cards = "".join(
        f'<div class="score"><div class="donut" style="--v:{float(value.replace(",", ".")) * 10}"><span>{value}</span></div>'
        f'<div><h4>{title}</h4><p>{text}</p></div></div>'
        for value, title, text in scores
    )
    return f'<div class="scorecard">{cards}</div>'


def design_block():
    scores = chart_card(
        "Дизайн",
        "Оценка текущих экранов",
        "9,5 / 5,5",
        design_scores(),
        "Сильный визуал чтения и слабая ясность библиотеки. Проблема не в красоте, а в иерархии, ложных функциях и отсутствии режимов под разные задачи.",
    )
    return scores + design_coverage()


def funnel_chart():
    rows = [
        ("Визиты", "75 000", 100, "100%"),
        ("Скачивания", "30 000", 40, "40%"),
        ("Активация", "18 000", 24, "60%"),
        ("Trial", "4 500", 6, "15%"),
        ("Покупки", "540", 0.9, "12% trial"),
    ]
    body = "".join(
        f'<div class="funnel-row"><span class="funnel-label">{label}</span>'
        f'<div class="funnel-track"><div class="funnel-fill" style="width:{width}%"></div></div>'
        f'<span class="funnel-pct">{value}<br><small>{pct}</small></span></div>'
        for label, value, width, pct in rows
    )
    return chart_card(
        "Воронка",
        "Путь от визита до покупки",
        "540 покупок на 75 тыс. визитов",
        f'<div class="funnel">{body}</div>',
        "Конверсия 1,8% от скачиваний и 12% от trial. Узкое место — не покупка, а активация и доверие: 60% скачавших не доходят до trial.",
    )


def pricing_cards():
    cards = [
        ("Free", "$0", "навсегда", "Читалка без ограничений", ["Все форматы", "TTS и перевод", "Локальные книги", "Без рекламы"]),
        ("Pro", "$29.99", "один раз", "Полный набор для чтения", ["Всё из Free", "EPUB / PDF / DJVU", "Аннотации и поиск", "Приоритетная поддержка"], True),
        ("Sync", "$39", "в год", "Книги и прогресс между устройствами", ["WebDAV / S3", "История и закладки", "До 5 устройств", "Экспорт данных"]),
        ("Education", "$299–1499", "в год", "Школы, вузы, библиотеки", ["MSI / GPO", "Счёт и DPA", "Онбординг", "Приоритет"]),
    ]
    html = []
    for plan, price, period, desc, features, *rest in cards:
        featured = " featured" if rest and rest[0] else ""
        items = "".join(f"<li>{feature}</li>" for feature in features)
        html.append(
            f'<div class="price-card{featured}"><span class="plan">{plan}</span>'
            f'<div class="price">{price} <small>{period}</small></div>'
            f'<div class="desc">{desc}</div><ul>{items}</ul></div>'
        )
    return chart_card(
        "Монетизация",
        "Четыре тарифа без подписки на чтение",
        "Pro — якорь",
        f'<div class="pricing-grid">{"".join(html)}</div>',
        "Запуск Pro за $19.99 для первых 500 покупателей. Подписка только на Sync — она честно оплачивает серверы и поддержку. Free остаётся полноценной читалкой.",
    )


def waterfall():
    rows = [
        ("Цена Pro", "$29.99", 100, ""),
        ("MoR", "−$2.00", 6.7, "neg"),
        ("VAT", "−$4.00", 13.3, "neg"),
        ("Возвраты", "−$0.96", 3.2, "neg"),
        ("Поддержка", "−$3.00", 10.0, "neg"),
        ("FX", "−$0.30", 1.0, "neg"),
        ("Чистыми", "$19.73", 65.8, "total"),
    ]
    body = "".join(
        f'<div class="wf-row {row_class}"><span class="wf-label">{label}</span>'
        f'<div class="wf-track"><div class="wf-fill" style="width:{width}%"></div></div>'
        f'<span class="wf-value">{value}</span></div>'
        for label, value, width, row_class in rows
    )
    return chart_card(
        "Юнит-экономика",
        "Куда исчезают $10 из $29.99",
        "Маржа 66%",
        body,
        "Без налога на прибыль и маркетинга. При цене $19.99 чистыми остаётся около $12.5–13 — поэтому запуск по низкой цене должен быть ограничен по времени и количеству.",
    )


def channel_heatmap():
    rows = [
        ("SEO и контент", [4, 5, 5, 2, 5]),
        ("Сообщества", [3, 4, 5, 4, 3]),
        ("Product Hunt / HN", [4, 3, 5, 5, 2]),
        ("Магазины приложений", [5, 4, 4, 3, 3]),
        ("Партнёры / affiliate", [3, 3, 3, 4, 3]),
        ("B2B outbound", [2, 5, 2, 2, 5]),
        ("Платный трафик", [5, 3, 1, 5, 4]),
        ("AppSumo / lifetime", [4, 2, 2, 4, 2]),
    ]
    head = "".join(f"<th>{h}</th>" for h in ["Канал", "Охват", "Намерение", "Дешевизна", "Скорость", "Контроль"])
    body_rows = []
    for name, scores in rows:
        cells = "".join(f'<td class="s{score}">{score}</td>' for score in scores)
        body_rows.append(f"<tr><td>{name}</td>{cells}</tr>")
    table = f'<table class="heatmap"><thead><tr>{head}</tr></thead><tbody>{"".join(body_rows)}</tbody></table>'
    legend = (
        '<div class="heat-legend">1 <i class="s1"></i><i class="s2"></i><i class="s3"></i><i class="s4"></i><i class="s5"></i> 5 · 5 = лучше</div>'
    )
    return chart_card(
        "Каналы",
        "Матрица приоритетов: где продавать",
        "5 = сильная сторона",
        table + legend,
        "SEO и сообщества дают контроль и дешёвый трафик, но медленные. Магазины — быстрый охват. Платный трафик опасен при ARPU $16–18. B2B — лучший контроль, но долгий цикл.",
    )


def risk_matrix():
    cells = {
        ("Высокое", "Низкая"): ["Магазины", "Юридические претензии"],
        ("Высокое", "Средняя"): ["UnRAR", "Доступность egui"],
        ("Высокое", "Высокая"): ["Бесплатные конкуренты", "Edge TTS", "SmartScreen", "Платежи RU", "Один основатель"],
        ("Среднее", "Низкая"): ["Валюта", "Железо"],
        ("Среднее", "Средняя"): ["Поддержка", "Репутация"],
        ("Среднее", "Высокая"): ["Качество macOS / Linux"],
        ("Низкое", "Низкая"): ["Мелкие баги"],
        ("Низкое", "Средняя"): ["Feature creep"],
        ("Низкое", "Высокая"): ["Пиратство"],
    }
    header = "<tr><th></th><th>Низкая</th><th>Средняя</th><th>Высокая</th></tr>"
    rows = []
    for impact in ["Высокое", "Среднее", "Низкое"]:
        row = [f"<th>{impact}</th>"]
        for probability in ["Низкая", "Средняя", "Высокая"]:
            names = cells[(impact, probability)]
            chips = "".join(f'<span class="risk-chip">{name}</span>' for name in names)
            cls = "hh" if impact == "Высокое" and probability == "Высокая" else "hm" if impact == "Высокое" else "mm" if impact == "Среднее" else "ll"
            row.append(f'<td class="{cls}">{chips}</td>')
        rows.append("<tr>" + "".join(row) + "</tr>")
    table = f'<table class="risk-table"><thead>{header}</thead><tbody>{"".join(rows)}</tbody></table>'
    return chart_card(
        "Риски",
        "Матрица: вероятность × влияние",
        "Красное — критично",
        table,
        "Столбцы — вероятность, строки — влияние. Самые опасные риски не технические: бесплатные конкуренты, Edge TTS, SmartScreen, платежи и ёмкость одного основателя.",
    )


def roadmap():
    items = [
        ("Месяцы 1–3", "Упаковка и запуск", "Free/Pro, сайт, checkout, SmartScreen, 5 тыс. скачиваний."),
        ("Месяцы 4–6", "Первые продажи", "150 продаж, 15 тыс. скачиваний, пилот B2B, аннотации."),
        ("Месяцы 7–9", "Масштаб", "Sync beta, Microsoft Store / Setapp, 300 продаж, 30 тыс. скачиваний."),
        ("Месяцы 10–12", "B2B и удержание", "600 продаж, 3 контракта, v1.2, 50 тыс. скачиваний."),
    ]
    body = "".join(
        f'<div class="tl-item"><span class="tl-dot"></span><div class="tl-date">{date}</div><h4>{title}</h4><p>{text}</p></div>'
        for date, title, text in items
    )
    return chart_card(
        "План",
        "12 месяцев: от упаковки до B2B",
        "20 часов в неделю",
        f'<div class="timeline">{body}</div>',
        "План реалистичен только при узком фокусе. Маркетинг — такое же узкое место, как разработка; без 30% времени на продажи цифры не достигаются.",
    )


def metrics():
    cards = [
        ("≥ 45%", "WAR · недельная активация"),
        ("25–35%", "D7 retention"),
        ("15–25%", "D30 retention"),
        ("8–12%", "Trial → paid"),
        ("< 5%", "Возвраты"),
        ("≥ 99,5%", "Crash-free sessions"),
        ("≥ 40", "NPS"),
        ("< 10 ч", "Поддержка в неделю"),
        ("< 30 дней", "Payback"),
    ]
    body = "".join(f'<div class="metric"><b>{value}</b><span>{label}</span></div>' for value, label in cards)
    return chart_card(
        "Метрики",
        "Приборная панель основателя",
        "Проверять еженедельно",
        f'<div class="metric-grid">{body}</div>',
        "Главная метрика — не выручка, а активация: доля скачавших, которые открыли книгу и вернулись на следующий день. Всё остальное — производные.",
    )


def quadrant():
    points = [
        ("Calibre", 22, 26, ""),
        ("Thorium", 24, 70, ""),
        ("Sumatra", 12, 79, ""),
        ("QuietReader", 77, 24, "primary"),
        ("Moon+", 86, 38, "secondary"),
        ("Readwise", 73, 69, ""),
        ("Speechify", 84, 79, ""),
    ]
    html = "".join(
        f'<div class="q-point {cls}" style="left:{x}%;top:{y}%">{name}</div>'
        for name, x, y, cls in points
    )
    body = (
        '<div class="quadrant-map">'
        '<span class="q-label tl">Бесплатно · много функций</span>'
        '<span class="q-label tr">Платно · много функций</span>'
        '<span class="q-label bl">Бесплатно · просто</span>'
        '<span class="q-label br">Платно · просто</span>'
        f"{html}</div>"
    )
    return chart_card(
        "Конкуренты",
        "Позиционная карта: где свободно",
        "Десктоп · чтение",
        body,
        "QuietReader не должен конкурировать с Calibre по функциям. Его позиция — платный, красивый, локальный инструмент для чтения, а не комбайн для библиотеки.",
    )


def sensitivity():
    rows = [
        ("Скачивания −50%", 4.3),
        ("Цена −$5", 2.7),
        ("Конверсия −0,5 п.п.", 2.4),
        ("B2B −2 сделки", 2.0),
        ("Возвраты +5 п.п.", 0.5),
    ]
    body = "".join(
        f'<div class="hbar-row"><span class="hbar-label">{label}</span><div class="hbar-track"><div class="hbar-fill neg" style="width:{(value / 4.3) * 100:.0f}%"></div></div><span class="hbar-value">−${value:.1f} тыс.</span></div>'
        for label, value in rows
    )
    return chart_card(
        "Чувствительность",
        "Что сильнее всего роняет выручку",
        "Базовый сценарий, год 1",
        body,
        "Падение скачиваний на 50% стоит около $4,3 тыс. в год 1 — больше, чем снижение цены на $5. Поэтому дистрибуция важнее оптимизации прайса.",
    )
