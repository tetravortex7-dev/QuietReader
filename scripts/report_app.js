(function () {
  var root = document.documentElement;
  root.classList.add("js");

  var themes = ["light", "sepia", "dark"];
  var themeLabels = { light: "Светлая", sepia: "Сепия", dark: "Тёмная" };
  var theme = "light";
  try {
    theme = localStorage.getItem("quietreader-report-theme") || "light";
  } catch (error) {
    theme = "light";
  }
  if (themes.indexOf(theme) === -1) {
    theme = "light";
  }

  function applyTheme(next) {
    theme = next;
    root.setAttribute("data-theme", next);
    var label = document.getElementById("themeLabel");
    if (label) {
      label.textContent = themeLabels[next];
    }
    var button = document.getElementById("themeButton");
    if (button) {
      button.setAttribute("aria-label", "Тема: " + themeLabels[next]);
    }
    try {
      localStorage.setItem("quietreader-report-theme", next);
    } catch (error) {}
  }

  var themeButton = document.getElementById("themeButton");
  if (themeButton) {
    themeButton.addEventListener("click", function () {
      applyTheme(themes[(themes.indexOf(theme) + 1) % themes.length]);
    });
  }
  applyTheme(theme);

  var progress = document.getElementById("progressValue");
  function updateProgress() {
    if (!progress) {
      return;
    }
    var height = document.documentElement.scrollHeight - document.documentElement.clientHeight;
    var value = height > 0 ? document.documentElement.scrollTop / height : 0;
    progress.style.transform = "scaleX(" + Math.min(1, Math.max(0, value)) + ")";
  }
  window.addEventListener("scroll", updateProgress, { passive: true });
  window.addEventListener("resize", updateProgress);
  updateProgress();

  var tocLinks = Array.prototype.slice.call(document.querySelectorAll(".toc-link"));
  var linkById = {};
  tocLinks.forEach(function (link) {
    var id = (link.getAttribute("href") || "").replace("#", "");
    if (id) {
      linkById[id] = link;
    }
  });

  var headingIds = {};
  document.querySelectorAll(".content h2[id], .content h3[id]").forEach(function (heading) {
    headingIds[heading.id] = true;
  });

  function setActive(id) {
    tocLinks.forEach(function (link) {
      link.classList.remove("active");
    });
    var active = linkById[id];
    if (active) {
      active.classList.add("active");
      var sidebar = document.querySelector(".sidebar");
      if (sidebar && sidebar.scrollHeight > sidebar.clientHeight && window.innerWidth > 1020) {
        var top = active.offsetTop;
        var bottom = top + active.offsetHeight;
        if (top < sidebar.scrollTop + 24 || bottom > sidebar.scrollTop + sidebar.clientHeight - 24) {
          sidebar.scrollTo({ top: Math.max(0, top - sidebar.clientHeight / 2), behavior: "smooth" });
        }
      }
    }
  }

  if ("IntersectionObserver" in window) {
    var headingObserver = new IntersectionObserver(function (entries) {
      var visible = entries
        .filter(function (entry) {
          return entry.isIntersecting && headingIds[entry.target.id];
        })
        .sort(function (a, b) {
          return a.boundingClientRect.top - b.boundingClientRect.top;
        });
      if (visible.length) {
        setActive(visible[0].target.id);
      }
    }, { rootMargin: "-12% 0px -72% 0px", threshold: 0 });
    document.querySelectorAll(".content h2[id], .content h3[id]").forEach(function (heading) {
      headingObserver.observe(heading);
    });
  }

  var search = document.getElementById("tocSearch");
  if (search) {
    search.addEventListener("input", function () {
      var query = search.value.trim().toLowerCase();
      tocLinks.forEach(function (link) {
        var text = (link.textContent || "").toLowerCase();
        link.style.display = query && text.indexOf(query) === -1 ? "none" : "";
      });
    });
  }

  var menuButton = document.getElementById("menuButton");
  var sidebar = document.querySelector(".sidebar");
  if (menuButton && sidebar) {
    menuButton.addEventListener("click", function () {
      sidebar.classList.toggle("open");
    });
    document.querySelectorAll(".sidebar .toc-link").forEach(function (link) {
      link.addEventListener("click", function () {
        sidebar.classList.remove("open");
      });
    });
    document.addEventListener("click", function (event) {
      if (window.innerWidth <= 1020 && sidebar.classList.contains("open")) {
        if (!sidebar.contains(event.target) && !menuButton.contains(event.target)) {
          sidebar.classList.remove("open");
        }
      }
    });
  }

  var topButton = document.getElementById("toTop");
  if (topButton) {
    topButton.addEventListener("click", function () {
      window.scrollTo({ top: 0, behavior: "smooth" });
    });
    window.addEventListener("scroll", function () {
      topButton.classList.toggle("show", window.scrollY > 900);
    }, { passive: true });
  }

  if ("IntersectionObserver" in window) {
    var chartObserver = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) {
          entry.target.classList.add("in");
          chartObserver.unobserve(entry.target);
        }
      });
    }, { threshold: 0.12 });
    document.querySelectorAll(".chart-card").forEach(function (card) {
      chartObserver.observe(card);
    });
  } else {
    document.querySelectorAll(".chart-card").forEach(function (card) {
      card.classList.add("in");
    });
  }

  var printButton = document.getElementById("printButton");
  if (printButton) {
    printButton.addEventListener("click", function () {
      window.print();
    });
  }
})();
