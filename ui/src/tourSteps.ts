import type { TourStep } from './tourState';

/** The steps of the tour on the home page. */
export function homeSteps(openDocs: (page: string) => void): TourStep[] {
  return [
    {
      title: 'Welcome to the Pack Creator',
      text: 'This is an editor for packs: the girls, scenes, behaviours and conversations that Pleasure Horizons reads. This short tour shows you round. You can leave it at any time with Skip, and replay it later from here or from the top bar of a project.'
    },
    {
      target: '[data-tour="home-new"]',
      title: 'Start a project',
      text: 'A project is one data pack: a folder the game reads. New project asks what the pack will hold (girls, scenes, traits, dialogue, or any mix) and makes the folders for you.'
    },
    {
      target: '[data-tour="home-open"]',
      title: 'Open a pack you already have',
      text: 'Pick a folder to open a pack made by hand, by a friend or by an older tool. Nothing in it is changed until you edit something.'
    },
    {
      target: '[data-tour="home-projects"]',
      title: 'Your projects',
      text: 'Projects you have opened are listed here, newest first. Removing one from the list never deletes its folder.'
    },
    {
      target: '[data-tour="home-docs"]',
      title: 'The documentation',
      text: 'Guides written for pack makers, and reference pages made from the game itself so they always match the version you play. Press F1 inside a project to open it any time.',
      action: { label: 'Open "Your first pack"', run: () => openDocs('first-pack') }
    },
    {
      title: 'Ready',
      text: 'Make a project or open one. A second, shorter tour will walk you round the workspace the first time a project opens.'
    }
  ];
}

/** The steps of the tour inside a project. */
export function workspaceSteps(openDocs: (page: string) => void): TourStep[] {
  return [
    {
      title: 'Your workspace',
      text: 'This is where a pack is built. The tour touches each part in turn. Nothing you do here is lost: everything saves as you type, and every file keeps its earlier versions.'
    },
    {
      target: '.sidebar',
      title: 'The file tree',
      text: 'Every file of the pack. Click to open, right-click to rename, duplicate, copy the path or delete (deleted files go to the Recycle Bin). You can drag files in from Explorer and move them around, and Ctrl+Z undoes a move.'
    },
    {
      target: '[data-tour="new"]',
      title: 'Make something new',
      text: 'Ctrl+N makes a new girl, scene, trait or dialogue from a template. A new girl opens a wizard that checks her models against her skeleton and writes all her files together.'
    },
    {
      target: '[data-tour="quick"]',
      title: 'Jump to a file',
      text: 'Ctrl+P opens a file by typing part of its name. It is quicker than the tree once a pack grows.'
    },
    {
      target: '[data-tour="find"]',
      title: 'Find and replace everywhere',
      text: 'Ctrl+Shift+F searches every file in the pack. Use it to rename an id or a sound in all the places it appears.'
    },
    {
      target: '.editor-area',
      title: 'The editor',
      text: 'Girl, scene, trait and dialogue files open as a Form made from the game\'s own rules, with a JSON switch above to see the same file as text. The two always agree. Models and animations open in a 3D viewer instead, and images in an image view.'
    },
    {
      target: '.problems',
      title: 'Problems',
      text: 'The pack is checked against the game\'s rules whenever you save: a wrong value, a missing model, a cue nothing fires. Click a problem to jump to it, and look for the quick fix. Aim for "No problems found".'
    },
    {
      target: '[data-tour="play"]',
      title: 'Try it in the game',
      text: 'Copies the pack into a world\'s datapacks folder. Open the world, run /reload, and look for your girl. For scenes, the Simulate button on a scene file plays it right here first.'
    },
    {
      target: '[data-tour="export"]',
      title: 'Share it',
      text: 'Writes the pack as a zip. Others drop it into a world\'s datapacks folder, and the game sends the models and sounds to players when they join.'
    },
    {
      target: '[data-tour="import"]',
      title: 'Bring in an old girl',
      text: 'Converts a girl from an older resource pack or profile into a pack here, and reports what it changed.'
    },
    {
      target: '[data-tour="blockbench"]',
      title: 'Blockbench',
      text: 'Installs Blockbench with the Strut Toolkit, on a profile of its own, for making models and animations. The viewer\'s Edit in Blockbench button opens the file you are looking at.'
    },
    {
      target: '[data-tour="log"]',
      title: 'The game log',
      text: 'Run the game, then open this: it lists what the game said about your pack, tied to the file each line names. It is the first place to look when something works here but not in the game.'
    },
    {
      target: '[data-tour="docs"]',
      title: 'Docs, any time',
      text: 'F1 opens the guides and the game\'s reference. Each kind of file has a guide with an example and a list of common problems.',
      action: { label: 'Open "Your first pack"', run: () => openDocs('first-pack') }
    },
    {
      target: '[data-tour="tour"]',
      title: 'Replay this tour',
      text: 'This button starts the tour again whenever you want it. The home page has one too.'
    },
    {
      target: '.statusbar',
      title: 'The status bar',
      text: 'Shows whether everything is saved, how many errors and warnings the pack has, and where your cursor is.'
    },
    {
      title: 'You are set',
      text: 'Try the walk-through: it takes you from an empty project to a girl in the game.',
      action: { label: 'Open "Your first pack"', run: () => openDocs('first-pack') }
    }
  ];
}
