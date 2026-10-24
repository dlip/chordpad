"use strict";
document.documentElement.classList.add("js");
const search = document.getElementById("search");
const cards = Array.from(document.querySelectorAll(".chord-card"), element => ({
  element,
  characters: JSON.parse(element.dataset.characters),
  labels: JSON.parse(element.dataset.labels),
}));
const layers = Array.from(document.querySelectorAll(".layer"));
const filters = Array.from(document.querySelectorAll("[data-filter]"));
const cells = Array.from(document.querySelectorAll(".binding-cell"));
for (const card of cards) {
  card.cells = cells.filter(cell => cell.closest(".layer") === card.element.closest(".layer") && cell.dataset.chord === card.element.dataset.chord);
  for (const cell of card.cells) cell.title = card.element.querySelector(".chord-notation").textContent;
}
let activeLayer = "all";

function filter() {
  const query = search.value;
  const literal = Array.from(query).length === 1;
  const words = query.trim().toLowerCase();
  const inLayer = card => activeLayer === "all" || card.element.closest(".layer").dataset.layer === activeLayer;
  const exactName = !literal && words !== "" && cards.some(card =>
    inLayer(card) && card.labels.some(label => label.toLowerCase() === words));
  let visible = 0;
  const matches = new Set();
  const instructions = [];
  for (const card of cards) {
    const match = !query || (literal
      ? card.characters.includes(query) || card.labels.includes(query)
      : exactName ? card.labels.some(label => label.toLowerCase() === words) : card.element.dataset.search.includes(words));
    card.element.hidden = !inLayer(card) || !match;
    if (!card.element.hidden) {
      visible++;
      for (const cell of card.cells) matches.add(cell);
      if (query && card.cells.length === 0) instructions.push(card.element.querySelector(".chord-notation").textContent);
    }
  }
  for (const cell of cells) {
    cell.classList.toggle("search-match", query !== "" && matches.has(cell));
    cell.classList.toggle("search-muted", query !== "" && !matches.has(cell));
  }
  for (const layer of layers) {
    const count = layer.querySelectorAll(".chord-card:not([hidden])").length;
    layer.hidden = (activeLayer !== "all" && layer.dataset.layer !== activeLayer) || (query !== "" && count === 0);
  }
  for (const button of filters) {
    button.setAttribute("aria-pressed", String(button.dataset.filter === activeLayer));
  }
  document.getElementById("result-count").textContent =
    `${visible} of ${cards.length} chords${literal ? " · exact character" : ""}${instructions.length ? " · " + instructions.join("; ") : ""}`;
  document.getElementById("no-results").hidden = visible !== 0;
}

for (const button of filters) button.addEventListener("click", () => {
  activeLayer = button.dataset.filter;
  filter();
});
search.addEventListener("input", filter);
document.getElementById("clear-search").addEventListener("click", () => {
  search.value = "";
  filter();
  search.focus();
});
search.addEventListener("keydown", event => {
  if (event.key === "Escape") { search.value = ""; filter(); }
});
document.getElementById("print").addEventListener("click", () => window.print());
filter();
