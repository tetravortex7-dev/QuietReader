const fs = require('fs');
const path = require('path');

const appData = process.env.APPDATA;
const libPath = path.join(appData, 'QuietReader', 'library.json');

if (!fs.existsSync(libPath)) {
    console.log('library.json not found');
    process.exit(0);
}

const lib = JSON.parse(fs.readFileSync(libPath, 'utf8'));
const now = Math.floor(Date.now() / 1000);
const today = Math.floor(now / 86400);

const sampleCozyBooks = [
    {
        path: 'C:\\Users\\Almas\\Documents\\QuietReader\\books\\Lavender_Room.epub',
        title: 'Лавандовая комната',
        author: 'Нина Георге',
        current_unit: 13,
        total_units: 20,
        last_opened_unix: now - 22 * 60,
        status: 'reading'
    },
    {
        path: 'C:\\Users\\Almas\\Documents\\QuietReader\\books\\Solaris.fb2',
        title: 'Солярис',
        author: 'Станислав Лем',
        current_unit: 7,
        total_units: 18,
        last_opened_unix: now - 3 * 3600,
        status: 'reading'
    },
    {
        path: 'C:\\Users\\Almas\\Documents\\QuietReader\\books\\Neuromancer.pdf',
        title: 'Нейромант',
        author: 'Уильям Гибсон',
        current_unit: 1,
        total_units: 10,
        last_opened_unix: now - 86400,
        status: 'reading'
    },
    {
        path: 'C:\\Users\\Almas\\Documents\\QuietReader\\books\\Master_i_Margarita.fb2',
        title: 'Мастер и Маргарита',
        author: 'Михаил Булгаков',
        current_unit: 18,
        total_units: 32,
        last_opened_unix: now - 2 * 86400,
        status: 'reading'
    },
    {
        path: 'C:\\Users\\Almas\\Documents\\QuietReader\\books\\Invisible_Cities.epub',
        title: 'Невидимые города',
        author: 'Итало Кальвино',
        current_unit: 5,
        total_units: 9,
        last_opened_unix: now - 3 * 86400,
        status: 'reading'
    },
    {
        path: 'C:\\Users\\Almas\\Documents\\QuietReader\\books\\Crime_and_Punishment.epub',
        title: 'Преступление и наказание',
        author: 'Фёдор Достоевский',
        current_unit: 400,
        total_units: 400,
        last_opened_unix: now - 4 * 86400,
        status: 'finished'
    },
    {
        path: 'C:\\Users\\Almas\\Documents\\QuietReader\\books\\The_Great_Gatsby.epub',
        title: 'Великий Гэтсби',
        author: 'Ф. Скотт Фицджеральд',
        current_unit: 180,
        total_units: 180,
        last_opened_unix: now - 5 * 86400,
        status: 'finished'
    },
    {
        path: 'C:\\Users\\Almas\\Documents\\QuietReader\\books\\Dune.fb2',
        title: 'Дюна',
        author: 'Фрэнк Герберт',
        current_unit: 120,
        total_units: 600,
        last_opened_unix: now - 6 * 86400,
        status: 'reading'
    },
    {
        path: 'C:\\Users\\Almas\\Documents\\QuietReader\\books\\Atomic_Habits.pdf',
        title: 'Атомные привычки',
        author: 'Джеймс Клир',
        current_unit: 300,
        total_units: 300,
        last_opened_unix: now - 7 * 86400,
        status: 'finished'
    },
    {
        path: 'C:\\Users\\Almas\\Documents\\QuietReader\\books\\1984.epub',
        title: '1984',
        author: 'Джордж Оруэлл',
        current_unit: 250,
        total_units: 250,
        last_opened_unix: now - 8 * 86400,
        status: 'finished'
    },
    {
        path: 'C:\\Users\\Almas\\Documents\\QuietReader\\books\\Flowers_for_Algernon.epub',
        title: 'Цветы для Элджернона',
        author: 'Дэниел Киз',
        current_unit: 80,
        total_units: 280,
        last_opened_unix: now - 9 * 86400,
        status: 'reading'
    },
    {
        path: 'C:\\Users\\Almas\\Documents\\QuietReader\\books\\Martian_Chronicles.fb2',
        title: 'Марсианские хроники',
        author: 'Рэй Брэдбери',
        current_unit: 15,
        total_units: 220,
        last_opened_unix: now - 10 * 86400,
        status: 'reading'
    }
];

const existingPaths = new Set(lib.entries.map(e => e.path));

for (const b of sampleCozyBooks) {
    const existing = lib.entries.find(e => e.path === b.path);
    if (existing) {
        Object.assign(existing, b);
    } else {
        lib.entries.push(b);
    }
}

// Ensure 7-day reading activity
if (!lib.daily_reading_seconds) {
    lib.daily_reading_seconds = {};
}
const activityMins = [45, 60, 30, 80, 50, 70, 42];
for (let i = 0; i < 7; i++) {
    const day = today - (6 - i);
    lib.daily_reading_seconds[day] = activityMins[i] * 60;
}

lib.mode = 'shelf';
lib.design = 'cozy';
lib.shelf_view = 'standard';

fs.writeFileSync(libPath, JSON.stringify(lib, null, 2), 'utf8');
console.log('Successfully enriched library.json with', lib.entries.length, 'books and 7-day activity');
