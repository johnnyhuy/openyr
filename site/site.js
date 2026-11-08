'use strict';
const raw = 'https://raw.githubusercontent.com/johnnyhuy/openyr/';
const gifs = raw + '1d3c9840cfa1293881ee093db9aec3e72a253235/proof/readme-features/';
const stills = raw + 'c7deba4c12ab133026c00570e4a2c78d7fa3fa65/proof/native-demo/';
const records = {
  setup: ['SKIRMISH SETUP', stills + 'skirmish.png', 'OpenYR skirmish setup with countries and a map preview', 'Choose your country, colors, starting position and battlefield. Two participants, nine Allied/Soviet countries.'],
  battle: ['BATTLEFIELD', gifs + 'world-camera.gif', 'OpenYR battlefield camera movement, zoom and pause controls', 'Retail terrain and units, native camera pan and zoom, and in-match controls.'],
  menu: ['MAIN MENU', gifs + 'main-menu.gif', 'Animated original-style OpenYR main menu', 'The animated shell, Single Player navigation and original-style command buttons.'],
  loading: ['MATCH LOADING', gifs + 'loading.gif', 'Skirmish setup, country loading artwork and entry into a match', 'Country artwork and the selected-map preview accompany match loading.'],
  save: ['SAVED MATCHES', gifs + 'save-name.gif', 'Selecting and replacing a save name before saving a match', 'Named saves, overwrite confirmation and native name-field shortcuts.']
};
const image = document.getElementById('record-image');
const link = document.getElementById('record-link');
const error = document.getElementById('media-error');
const buttons = document.querySelectorAll('[data-record]');
image.addEventListener('error', () => { error.hidden = false; });
image.addEventListener('load', () => { error.hidden = true; });
if (image.complete && image.naturalWidth === 0) error.hidden = false;
buttons.forEach(button => button.addEventListener('click', () => {
  const [title, src, alt, caption] = records[button.dataset.record];
  image.src = src;
  image.alt = alt;
  link.href = src;
  link.setAttribute('aria-label', 'Open the full ' + title.toLowerCase() + ' capture');
  document.getElementById('record-name').textContent = title;
  document.getElementById('record-caption').textContent = caption;
  buttons.forEach(item => item.setAttribute('aria-pressed', String(item === button)));
}));
document.querySelector('.record-controls').hidden = false;
