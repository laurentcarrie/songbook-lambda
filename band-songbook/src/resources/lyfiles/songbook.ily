%% Shared LilyPond library for the whole song corpus.
%%
%% This copy is compiled into the band-songbook binary and written to
%% <sandbox>/songs/songbook.ily on every build, so a corpus that never wrote
%% one still gets the macros. A songbook.ily beside the corpus settings.yml
%% replaces it verbatim - that is the hook for adding your own.
%%
%% Songs reach it with a relative include, from the generated macros.ly:
%%
%%     \include "../../songbook.ily"
%%
%% Put your own copy in the corpus if you want that to resolve from
%% songs/<artist>/<song>/ as well as from the sandbox mirror: that is what
%% lets a .ly compile straight from songs/ in the editor.
%%
%% Only per-song values belong in the generated macros.ly - songtempo.

% Guitar articulation marks. \mypull and \myrelease take the grace note and the
% note it slurs into; \mypulled marks a single note.
mypull =
#(define-scheme-function
  (na nb)
  (ly:music? ly:music?)
  #{
    \grace {$na ^\markup {\char ##x27B6 }} $nb
  #})

mypulled =
#(define-scheme-function
  (na)
  (ly:music?)
  #{
    $na ^\markup {\char ##x27B6 }
  #})

myrelease =
#(define-scheme-function
  (na nb)
  (ly:music? ly:music?)
  #{
    \grace {$na ^\markup {\char ##x27B4 }} $nb
  #})

% Red tick on every beat, as a transcription guide. Takes the number of 4/4
% bars to cover - pass the bar count of the score it sits beside. Odd beats (1
% and 3) get a square, even beats (2 and 4) a cross, so the backbeat is legible
% at a glance. Put it in a Dynamics context next to the TabStaff: a plain second
% Voice inside a TabStaff is not a TabVoice and wrecks the tablature. Before the
% staff puts the marks above it, after it puts them below.
songbookBeatMarks =
#(define-music-function (bars) (integer?)
   #{
     \repeat unfold $bars {
       s4^\markup { \with-color #red \bold "▮" }
       s4^\markup { \with-color #red \bold "✖" }
       s4^\markup { \with-color #red \bold "▮" }
       s4^\markup { \with-color #red \bold "✖" }
     }
   #})

% Basic backbeat: bass drum on 1 and 3, snare on 2 and 4, hi-hat on every
% eighth. Takes the number of 4/4 bars. Two voices, because the hi-hat runs
% against the drum/snake pattern:
%
%     \new DrumStaff { \songbookDrums 8 }
%
% It is real drum music, so it plays in the MIDI (channel 10) as well as being
% engraved - unlike \songbookBeatMarks, which is a purely visual guide.
songbookDrums =
#(define-music-function (bars) (integer?)
   #{
     \drummode {
       \repeat unfold $bars {
         << \new DrumVoice { \voiceOne hh8 hh hh hh hh hh hh hh }
            \new DrumVoice { \voiceTwo bd4 sn4 bd4 sn4 } >>
       }
     }
   #})
