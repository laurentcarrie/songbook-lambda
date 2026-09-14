songtempo = {{{song.info.tempo}}}

% The corpus-wide library (articulation marks, songbookBeatMarks) lives in
% songs/songbook.ily, next to settings.yml - so this relative path resolves
% from every song directory. band-songbook always puts it in the sandbox:
% the corpus copy if there is one, the library shipped in the binary if not.
% Pulling it in here means a song gets the whole library just by being built,
% with nothing to add to its own .ly files.
\include "../../songbook.ily"
