function getTheme() {
    let theme = localStorage.getItem('theme');
    if (theme === null) {
        theme = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
        localStorage.setItem('theme', theme);
    }
    return theme;
}

function applyTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
}

function toggleTheme() {
    let theme = getTheme() === 'light' ? 'dark' : 'light'
    localStorage.setItem('theme', theme);
    applyTheme(theme);
}

applyTheme(getTheme());