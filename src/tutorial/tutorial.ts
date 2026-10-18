import Shepherd, { type StepOptionsButton, type Tour } from 'shepherd.js';
import 'shepherd.js/dist/css/shepherd.css';
import './tutorialStyles.css';

const TUTORIAL_SEEN_KEY = 'colori-tutorial-seen';

let tour: Tour | null = null;

/** Scroll the game-content container to bring an element into view */
function scrollGameContent(el: HTMLElement | undefined) {
  const container = document.querySelector('.game-content');
  if (container && el) {
    const containerRect = container.getBoundingClientRect();
    const elRect = el.getBoundingClientRect();
    const scrollTop = container.scrollTop + (elRect.top - containerRect.top) - 60;
    container.scrollTo({ top: Math.max(0, scrollTop), behavior: 'smooth' });
  }
}

function createTour(): Tour {
  const newTour = new Shepherd.Tour({
    useModalOverlay: true,
    defaultStepOptions: {
      scrollTo: true,
      scrollToHandler: (el: HTMLElement) => scrollGameContent(el),
      cancelIcon: { enabled: true },
      classes: 'colori-shepherd-step',
    },
  });

  const backButton: StepOptionsButton = {
    text: 'Back',
    action() { return newTour.back(); },
    classes: 'shepherd-button-secondary',
  };

  const nextButton: StepOptionsButton = {
    text: 'Next',
    action() { return newTour.next(); },
    classes: 'shepherd-button-primary',
  };

  const skipButton: StepOptionsButton = {
    text: 'Skip',
    action() { return newTour.cancel(); },
    classes: 'shepherd-button-skip',
  };

  newTour.addStep({
    id: 'welcome',
    title: 'Welcome to Colori!',
    text: 'You are a vendecolori, a Renaissance Venetian color seller, competing to earn the most points by gathering pigments and materials and selling them to buyers.',
    buttons: [skipButton, nextButton],
  });

  newTour.addStep({
    id: 'buyers-board',
    title: "The Buyer's Board",
    text: "Shared by everyone. The seven sell cards are buyers: each wants a set of colors and usually a material, and the seal shows its points. You can't sell to them here; at the start of each round you claim them onto your own board. The circles along the top track the rounds and what each one brings; point at one to see.",
    attachTo: { element: '.market', on: 'right' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'player-board',
    title: 'Your Board',
    text: "Everything you own sits on your board, and the numbers on it are your stock. The tabs along the top switch to the other players' boards.",
    attachTo: { element: '.board-area', on: 'left' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'color-wheel',
    title: 'Color Wheel',
    text: 'The number by each color is how many you have. You start with one Red, Yellow and Blue. Mixing two colors two steps apart makes the color between them. The middle holds your wild primaries: each stands in for a Red, Yellow or Blue you lack, and is worth half a point at the end.',
    attachTo: { element: '.board-area [data-tour="wheel"]', on: 'right' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'materials',
    title: 'Materials and Boats',
    text: 'Your stored Paintings, Ceramics and Textiles. Most buyers want one. The gondola in the middle counts your boats: a boat stands in for a material you lack, and is worth half a point at the end.',
    attachTo: { element: '.board-area [data-tour="materials"]', on: 'right' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'buyers',
    title: 'Your Buyers',
    text: 'The buyers you have claimed: one slot now, a second from round 2 and a third from round 3. You can only sell to these, and a green check appears on one when you hold everything it asks for. Sold buyers go face down onto Completed Orders.',
    attachTo: { element: '.board-area [data-tour="buyers"]', on: 'left' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'ducats',
    title: 'Ducats',
    text: 'Each ducat is worth a point. You start with one and collect more in rounds 4, 6 and 7. On your turn you can also spend one, through the strip beside the purse, for a single Workshop pick, a Mix or a Sale. The strip lights up when you can.',
    attachTo: { element: '.board-area [data-tour="ducats"]', on: 'top' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'deck',
    title: 'Your Deck',
    text: 'Each round your workshop is topped up to five cards from here. When your turn ends, the workshop goes to the bottom, so a card you use will not come back until the rest have been drawn. Click it to see what is in it.',
    attachTo: { element: '.board-area [data-tour="deck"]', on: 'right' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'round-structure',
    title: 'Each Round',
    text: 'Each round has three phases: Buyers, Draft and Action. The game lasts 7 rounds.',
    attachTo: { element: '.round-indicator', on: 'bottom' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'buyers-phase',
    title: 'Buyers Phase',
    text: "Players take turns claiming one sell card at a time until every open slot is full: click one on the Buyer's Board, or the sell deck to take its top card unseen.",
    attachTo: { element: '.market', on: 'right' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'draft-phase',
    title: 'Draft Phase',
    text: "You're dealt 4 cards. Keep one and the rest pass on; after 3 picks the last card passed to you is yours too. Drafted cards tuck under the top of your board, showing the banner that says what trashing them does.",
    attachTo: { element: '.board-area [data-tour="drafted"]', on: 'bottom' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'workshop',
    title: 'Workshop',
    text: 'Your workshop cards tuck under the bottom edge, so the half that shows is what each gives when workshopped: colors, materials or an action. A Workshop ability lets you pick some of them.',
    attachTo: { element: '.board-area [data-tour="workshop"]', on: 'top' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'action-phase',
    title: 'Action Phase',
    text: 'Everyone plays at once. Click a drafted card to trash it and use its ability: Workshop, Mix, Sell, or move a workshop card up to your draft pool. This panel says what each ability needs, and the board lights up what you can click. Cards you keep move into your workshop, and your used workshop goes to the bottom of your deck.',
    attachTo: { element: '.controls', on: 'right' },
    buttons: [backButton, nextButton],
  });

  newTour.addStep({
    id: 'end-game',
    title: 'Good Luck!',
    text: 'The game ends after 7 rounds. Most points wins: the points on the buyers you sold to, a point for each ducat, and half a point for each wild primary and boat. Rest the pointer on a card, or press and hold it, to see all of it. Good luck, vendecolori!',
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

  // Wait for the boards to be in the DOM
  await waitForElement('.board-area [data-tour="wheel"]');

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
