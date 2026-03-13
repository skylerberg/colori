import Shepherd from 'shepherd.js';
import 'shepherd.js/dist/css/shepherd.css';
import './tutorialStyles.css';

const TUTORIAL_SEEN_KEY = 'colori-tutorial-seen';

let tour: Shepherd.Tour | null = null;

/** Scroll the game-content container to bring an element into view */
function scrollGameContent(el: HTMLElement) {
  const container = document.querySelector('.game-content');
  if (container) {
    const containerRect = container.getBoundingClientRect();
    const elRect = el.getBoundingClientRect();
    const scrollTop = container.scrollTop + (elRect.top - containerRect.top) - 60;
    container.scrollTo({ top: Math.max(0, scrollTop), behavior: 'smooth' });
  }
}

function createTour(): Shepherd.Tour {
  const newTour = new Shepherd.Tour({
    useModalOverlay: true,
    defaultStepOptions: {
      scrollTo: false,
      scrollToHandler: (el: HTMLElement) => scrollGameContent(el),
      cancelIcon: { enabled: true },
      classes: 'colori-shepherd-step',
    },
  });

  const backButton: Shepherd.Step.StepOptionsButton = {
    text: 'Back',
    action() { return newTour.back(); },
    classes: 'shepherd-button-secondary',
  };

  const nextButton: Shepherd.Step.StepOptionsButton = {
    text: 'Next',
    action() { return newTour.next(); },
    classes: 'shepherd-button-primary',
  };

  const skipButton: Shepherd.Step.StepOptionsButton = {
    text: 'Skip',
    action() { return newTour.cancel(); },
    classes: 'shepherd-button-skip',
  };

  newTour.addStep({
    id: 'welcome',
    title: 'Welcome to Colori!',
    text: 'You are a Venetian dye trader competing to earn the most ducats (gold coins) by getting pigments and materials then completing sell cards.',
    buttons: [skipButton, nextButton],
  });

  newTour.addStep({
    id: 'sell-card-display',
    title: 'Sell Cards',
    text: 'These are the available sell cards. Each sell card requires a specific material (Textiles, Ceramics, or Paintings) and a set of colors. Completing a sell card earns you ducats.',
    attachTo: { element: '.sell-card-display', on: 'bottom' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'color-wheel',
    title: 'Color Wheel',
    text: 'This is your color wheel. It tracks the colors you have stored. You start with 1 Red, 1 Yellow, and 1 Blue. Store more colors by workshopping dye cards, or mix two adjacent colors to create the color between them.',
    attachTo: { element: '.color-wheel-panel', on: 'top' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'materials',
    title: 'Materials',
    text: 'These are your stored materials. To sell to a sell card, you need both the right material AND the right colors. Gain materials by workshopping material cards.',
    attachTo: { element: '.materials-panel', on: 'top' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'player-stats',
    title: 'Player Stats',
    text: 'Track your ducats, and your personal deck and discard pile here. Tap on Deck or Discard to see their contents.',
    attachTo: { element: '.player-stats', on: 'bottom' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'round-structure',
    title: 'Round Structure',
    text: 'Each round has three phases: first, 5 cards are drawn from your personal deck into your workshop. Then comes the Draft, followed by the Action Phase.',
    attachTo: { element: '.top-info-bar', on: 'bottom' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'draft-phase',
    title: 'Draft Phase',
    text: "During the Draft Phase, you're dealt 5 cards from a shared draft deck. Pick 1 card, then pass your remaining cards to the left. You'll make 4 picks — the 5th card is destroyed without triggering its ability.",
    attachTo: { element: '.draft-section', on: 'bottom' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'workshop',
    title: 'Workshop',
    text: 'These are your workshop cards — drawn from your personal deck at the start of each round. During the Action Phase, the Workshop ability lets you activate these: dye cards store their colors, material cards store their materials, and action cards trigger special abilities like gaining ducats or drawing cards.',
    attachTo: { element: '.workshop-section', on: 'top' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'drafted-cards',
    title: 'Drafted Cards',
    text: "Cards you've drafted appear here. During the Action Phase, you can destroy these cards one at a time to trigger their abilities. Any cards you don't destroy go to your discard pile, building up your personal deck over time.",
    attachTo: { element: '.drafted-section', on: 'bottom' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'action-phase-concept',
    title: 'Action Phase',
    text: 'After drafting, each player takes a turn. On your turn, destroy drafted cards one at a time to trigger abilities like Workshop (store colors/materials), Mix Colors (combine two adjacent colors into one), Sell (complete a sell card), Draw Cards, or Destroy Cards (chain reactions). When done, remaining drafted and workshop cards go to your discard pile.',
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'selling-explained',
    title: 'Selling',
    text: 'To sell: choose a sell card, spend 1 stored material of the required type, and pay the color cost from your wheel. The sell card goes to your completed sell cards, earning you ducats.',
    attachTo: { element: '.sell-card-display', on: 'bottom' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'end-game',
    title: 'Good Luck!',
    text: 'The game ends after 20 rounds, or after completing a round where any player reaches 16 ducats. Most ducats wins. Good luck, vendecolori!',
    buttons: [
      backButton,
      {
        text: 'Start Playing',
        action() {
          localStorage.setItem(TUTORIAL_SEEN_KEY, 'true');
          return newTour.complete();
        },
        classes: 'shepherd-button-primary',
      },
    ],
  });

  newTour.on('cancel', () => {
    localStorage.setItem(TUTORIAL_SEEN_KEY, 'true');
  });

  return newTour;
}

/** Wait for a DOM element to exist, with timeout */
function waitForElement(selector: string, timeout = 3000): Promise<boolean> {
  return new Promise(resolve => {
    if (document.querySelector(selector)) return resolve(true);
    const observer = new MutationObserver(() => {
      if (document.querySelector(selector)) {
        observer.disconnect();
        resolve(true);
      }
    });
    observer.observe(document.body, { childList: true, subtree: true });
    setTimeout(() => { observer.disconnect(); resolve(false); }, timeout);
  });
}

export async function startTutorial(force = false): Promise<void> {
  if (!force && localStorage.getItem(TUTORIAL_SEEN_KEY) === 'true') {
    return;
  }

  // Cancel any existing tour
  if (tour) {
    tour.cancel();
    tour = null;
  }

  // Wait for key game elements to be in the DOM
  await waitForElement('.sell-card-display');

  tour = createTour();
  tour.start();
}

export function cancelTutorial(): void {
  if (tour) {
    tour.cancel();
    tour = null;
  }
}

export function isTutorialActive(): boolean {
  return tour?.isActive() ?? false;
}

export function resetTutorial(): void {
  localStorage.removeItem(TUTORIAL_SEEN_KEY);
}
