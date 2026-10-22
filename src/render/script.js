"use strict";
document.documentElement.classList.add("js");
const search = document.getElementById("search");
const cards = Array.from(document.querySelectorAll(".chord-card"));
const layers = Array.from(document.querySelectorAll(".layer"));
const filters = Array.from(document.querySelectorAll("[data-filter]"));
const preview = document.getElementById("preview-diagram");
const original = preview.firstElementChild.cloneNode(true);
const title = document.getElementById("preview-title");
const instruction = document.getElementById("preview-instruction");
const reset = document.getElementById("reset-preview");
const originalInstruction = instruction.textContent;
let activeLayer = "all";
let selected = null;

function showOverview() {
  if (selected) {
    selected.classList.remove("selected");
    selected.setAttribute("aria-pressed", "false");
  }
  selected = null;
  preview.replaceChildren(original.cloneNode(true));
  title.textContent = "BASE CONTROLS";
  instruction.textContent = originalInstruction;
  reset.disabled = true;
}

function showCard(card) {
  if (selected) {
    selected.classList.remove("selected");
    selected.setAttribute("aria-pressed", "false");
  }
  selected = card;
  card.classList.add("selected");
  card.setAttribute("aria-pressed", "true");
  preview.replaceChildren(card.querySelector("svg").cloneNode(true));
  title.textContent = card.dataset.label;
  instruction.textContent = card.querySelector(".chord-notation").textContent;
  reset.disabled = false;
}

function filter() {
  const query = search.value;
  const literal = Array.from(query).length === 1;
  const words = query.trim().toLowerCase();
  const exactName = !literal && words !== "" && cards.some(card =>
    (activeLayer === "all" || card.closest(".layer").dataset.layer === activeLayer) &&
    card.dataset.label.toLowerCase() === words);
  const visible = [];
  for (const layer of layers) {
    const inLayer = activeLayer === "all" || activeLayer === layer.dataset.layer;
    let count = 0;
    for (const card of layer.querySelectorAll(".chord-card")) {
      const match = !query || (literal
        ? card.dataset.character === query || (!card.dataset.character && card.dataset.label === query)
        : exactName ? card.dataset.label.toLowerCase() === words : card.dataset.search.includes(words));
      card.hidden = !inLayer || !match;
      if (!card.hidden) { count++; visible.push(card); }
    }
    layer.hidden = !inLayer || (query !== "" && count === 0);
  }
  for (const button of filters) {
    button.setAttribute("aria-pressed", String(button.dataset.filter === activeLayer));
  }
  document.getElementById("result-count").textContent =
    `${visible.length} of ${cards.length} bindings${literal ? " · exact character" : ""}`;
  document.getElementById("no-results").hidden = visible.length !== 0;
  if (visible.length === 1) showCard(visible[0]);
  else if (selected && selected.hidden) showOverview();
}

for (const card of cards) card.addEventListener("click", () => showCard(card));
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
reset.addEventListener("click", showOverview);
document.getElementById("print").addEventListener("click", () => window.print());
reset.disabled = true;
filter();
