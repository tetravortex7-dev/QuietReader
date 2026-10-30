use crate::core::library::{
    LibraryEntry, LibraryMode, LibraryReadingStatus, LibraryStore, ShelfViewMode, UiDesignMode,
};
use std::fs;
use std::path::{Path, PathBuf};

pub const WELCOME_BOOK_FILENAME: &str = "welcome_guide.fb2";
pub const WELCOME_BOOK_TITLE: &str = "Добро пожаловать в QuietReader";
pub const WELCOME_BOOK_AUTHOR: &str = "QuietReader";
pub const WELCOME_BOOK_CONTENT: &str = include_str!("../../assets/welcome_guide.fb2");

pub fn ensure_welcome_book(library_path: &Path, library: &mut LibraryStore) -> bool {
    if !library.entries.is_empty() {
        return false;
    }
    let Some(dir) = library_path.parent() else {
        return false;
    };
    let path = dir.join(WELCOME_BOOK_FILENAME);
    if !path.is_file() {
        if fs::create_dir_all(dir).is_err() {
            return false;
        }
        if fs::write(&path, WELCOME_BOOK_CONTENT).is_err() {
            return false;
        }
    }
    library.ensure_book(
        &path,
        WELCOME_BOOK_TITLE,
        WELCOME_BOOK_AUTHOR,
        crate::core::license::unix_now(),
    )
}

pub fn resolve_books_dir() -> PathBuf {
    let repo_books = PathBuf::from(r"C:\Users\Almas\Documents\QuietReader\books");
    if repo_books.exists() {
        return repo_books;
    }
    let candidates = [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("books"))),
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(|d| d.join("books"))
        }),
        std::env::current_dir().ok().map(|d| d.join("books")),
        Some(PathBuf::from("books")),
    ];
    for cand in candidates.into_iter().flatten() {
        if cand.exists() {
            return cand;
        }
    }
    repo_books
}

pub fn ensure_cozy_showcase_library(library: &mut LibraryStore) -> bool {
    let full_books_dir = resolve_books_dir();
    let now = crate::core::license::unix_now();
    let day = crate::core::library::unix_day_index(now);

    let first_is_cities = library
        .entries
        .iter()
        .any(|e| e.title == "Невидимые города");
    let has_lavender = library
        .entries
        .iter()
        .any(|e| e.title == "Лавандовая комната");

    let reading_count = library
        .entries
        .iter()
        .filter(|e| e.status == LibraryReadingStatus::Reading)
        .count();
    let finished_count = library
        .entries
        .iter()
        .filter(|e| e.status == LibraryReadingStatus::Finished)
        .count();

    if first_is_cities
        && has_lavender
        && library.entries.len() == 48
        && reading_count == 3
        && finished_count == 12
        && library.current_streak_days(day) >= 14
        && (library.total_reading_minutes() as f32 / 60.0 - 38.5).abs() < 0.2
        && library.mode == LibraryMode::Shelf
        && library.shelf_view == ShelfViewMode::Standard
        && library.design == UiDesignMode::Cozy
    {
        return false;
    }

    library.mode = LibraryMode::Shelf;
    library.shelf_view = ShelfViewMode::Standard;
    library.design = UiDesignMode::Cozy;
    library.library_folder = Some(full_books_dir.clone());

    let showcase_items: [(&str, &str, &str, LibraryReadingStatus, usize, usize, u64); 14] = [
        (
            "Invisible_Cities.epub",
            "Невидимые города",
            "Итало Кальвино",
            LibraryReadingStatus::Finished,
            9,
            10,
            now - 100,
        ),
        (
            "Dune.epub",
            "Дюна",
            "Фрэнк Герберт",
            LibraryReadingStatus::Unread,
            81,
            100,
            now - 200,
        ),
        (
            "Left_Hand_of_Darkness.pdf",
            "Левая рука Тьмы",
            "Урсула Ле Гуин",
            LibraryReadingStatus::Finished,
            19,
            20,
            now - 300,
        ),
        (
            "Watchmen.cbz",
            "Хранители",
            "Алан Мур, Дэйв Гиббонс",
            LibraryReadingStatus::Unread,
            23,
            100,
            now - 400,
        ),
        (
            "Ficciones.fb2",
            "Вымышленные истории",
            "Хорхе Луис Борхес",
            LibraryReadingStatus::Finished,
            15,
            16,
            now - 500,
        ),
        (
            "Poetics_of_Space.epub",
            "Поэтика пространства",
            "Гастон Башляр",
            LibraryReadingStatus::Unread,
            54,
            100,
            now - 600,
        ),
        (
            "Norwegian_Wood.epub",
            "Норвежский лес",
            "Харуки Мураками",
            LibraryReadingStatus::Finished,
            19,
            20,
            now - 700,
        ),
        (
            "Dispossessed.fb2",
            "Обделённые",
            "Урсула Ле Гуин",
            LibraryReadingStatus::Unread,
            11,
            100,
            now - 800,
        ),
        (
            "Zen_Motorcycle.pdf",
            "Дзен и искусство ухода за мотоциклом",
            "Роберт Пирсиг",
            LibraryReadingStatus::Finished,
            31,
            32,
            now - 900,
        ),
        (
            "Piranesi.epub",
            "Пиранези",
            "Сюзанна Кларк",
            LibraryReadingStatus::Finished,
            15,
            16,
            now - 1000,
        ),
        (
            "Memory_Police.epub",
            "Полиция памяти",
            "Ёко Огава",
            LibraryReadingStatus::Unread,
            35,
            100,
            now - 1100,
        ),
        (
            "Lavender_Room.epub",
            "Лавандовая комната",
            "Нина Георге",
            LibraryReadingStatus::Reading,
            13,
            19,
            now - 1320,
        ),
        (
            "Solaris.fb2",
            "Солярис",
            "Станислав Лем",
            LibraryReadingStatus::Reading,
            7,
            17,
            now - 10800,
        ),
        (
            "Neuromancer.pdf",
            "Нейромант",
            "Уильям Гибсон",
            LibraryReadingStatus::Reading,
            1,
            10,
            now - 86400,
        ),
    ];

    let mut new_entries = Vec::with_capacity(48);

    for (filename, title, author, status, cur, tot, opened) in showcase_items {
        let path = full_books_dir.join(filename);
        new_entries.push(LibraryEntry {
            path,
            title: title.to_string(),
            author: author.to_string(),
            added_at_unix: opened - 10000,
            last_opened_unix: opened,
            status,
            current_unit: cur,
            total_units: tot,
        });
    }

    let extra_titles = [
        "Мастер и Маргарита",
        "Преступление и наказание",
        "Великий Гэтсби",
        "1984",
        "Атомные привычки",
        "Сто лет одиночества",
        "Убийство в Восточном экспрессе",
        "Цветы для Элджернона",
        "Марсианские хроники",
        "Маленький принц",
        "Три товарища",
        "Над пропастью во ржи",
        "Герой нашего времени",
        "Отцы и дети",
        "Евгений Онегин",
        "Анна Каренина",
        "Идиот",
        "Братья Карамазовы",
        "Собачье сердце",
        "Белая гвардия",
        "Мёртвые души",
        "Ревизор",
        "Тарас Бульба",
        "Шинель",
        "Нос",
        "Вий",
        "Вечера на хуторе близ Диканьки",
        "Портрет Дориана Грея",
        "О дивный новый мир",
        "451 градус по Фаренгейту",
        "Бойцовский клуб",
        "Автостопом по галактике",
        "Властелин колец",
        "Хоббит",
    ];

    for (idx, title) in extra_titles.iter().enumerate() {
        let path = full_books_dir.join(format!("Extra_Book_{}.epub", idx));
        let is_fin = idx < 6;
        new_entries.push(LibraryEntry {
            path,
            title: (*title).to_string(),
            author: "Мировая литература".to_string(),
            added_at_unix: now - 500000 - idx as u64,
            last_opened_unix: now - 500000 - idx as u64,
            status: if is_fin {
                LibraryReadingStatus::Finished
            } else {
                LibraryReadingStatus::Unread
            },
            current_unit: if is_fin { 20 } else { 0 },
            total_units: if is_fin { 20 } else { 100 },
        });
    }

    library.entries = new_entries;
    library.daily_reading_seconds.clear();

    for d in (day.saturating_sub(13))..=(day.saturating_sub(7)) {
        library.record_reading_seconds(d, 16568);
    }
    library.record_reading_seconds(day.saturating_sub(6), 2700);
    library.record_reading_seconds(day.saturating_sub(5), 3600);
    library.record_reading_seconds(day.saturating_sub(4), 1800);
    library.record_reading_seconds(day.saturating_sub(3), 4800);
    library.record_reading_seconds(day.saturating_sub(2), 3000);
    library.record_reading_seconds(day.saturating_sub(1), 4200);
    library.record_reading_seconds(day, 2520);

    true
}
