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
let activeLayer = "all";
let searchState = null;

function filter() {
  const query = search.value;
  if (query && !searchState) {
    searchState = layers.map(layer => {
      const details = layer.querySelector(".binding-details");
      return [details, details.open];
    });
  } else if (!query && searchState) {
    for (const [details, open] of searchState) details.open = open;
    searchState = null;
  }
  const literal = Array.from(query).length === 1;
  const words = query.trim().toLowerCase();
  const inLayer = card => activeLayer === "all" || card.element.closest(".layer").dataset.layer === activeLayer;
  const exactName = !literal && words !== "" && cards.some(card =>
    inLayer(card) && card.labels.some(label => label.toLowerCase() === words));
  let visible = 0;
  for (const card of cards) {
    const match = !query || (literal
      ? card.characters.includes(query) || card.labels.includes(query)
      : exactName ? card.labels.some(label => label.toLowerCase() === words) : card.element.dataset.search.includes(words));
    card.element.hidden = !inLayer(card) || !match;
    card.element.classList.toggle("search-match", query !== "" && !card.element.hidden);
    if (!card.element.hidden) visible++;
  }
  for (const layer of layers) {
    const count = layer.querySelectorAll(".chord-card:not([hidden])").length;
    layer.hidden = (activeLayer !== "all" && layer.dataset.layer !== activeLayer) || (query !== "" && count === 0);
    if (query) layer.querySelector(".binding-details").open = true;
  }
  for (const button of filters) {
    button.setAttribute("aria-pressed", String(button.dataset.filter === activeLayer));
  }
  document.getElementById("result-count").textContent =
    `${visible} of ${cards.length} chords${literal ? " · exact character" : ""}`;
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
let printState = [];
window.addEventListener("beforeprint", () => {
  printState = Array.from(document.querySelectorAll(".binding-details"), details => [details, details.open]);
  for (const [details] of printState) details.open = true;
});
window.addEventListener("afterprint", () => {
  for (const [details, open] of printState) details.open = open;
});
document.getElementById("print").addEventListener("click", () => window.print());
filter();
