const desktop = document.querySelector('.desktop');
const controls = document.querySelector('.demo-controls');

if (desktop && controls) {
  for (const button of controls.querySelectorAll('button[data-effect]')) {
    button.addEventListener('click', () => {
      const disabled = desktop.classList.toggle(button.dataset.effect);
      button.setAttribute('aria-pressed', String(!disabled));
    });
  }
  controls.hidden = false;
}
