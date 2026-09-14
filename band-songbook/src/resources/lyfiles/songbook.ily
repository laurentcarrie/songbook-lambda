%% Shared LilyPond library for the whole song corpus.
%%
%% This is a real, versioned file - unlike macros.ly, which band-songbook
%% generates per song and which therefore only exists under sandbox/. Songs
%% reach it with a relative include:
%%
%%     \include "../../songbook.ily"
%%
%% which resolves identically from songs/<artist>/<song>/ and from its sandbox
%% mirror, because band-songbook copies this file next to settings.yml. That is
%% what lets a .ly compile straight from songs/ in the editor with the real
%% macros, not a stand-in.
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
% bars to cover - pass the bar count of the score it sits beside. Put it in a
% Dynamics context next to the TabStaff: a plain second Voice inside a TabStaff
% is not a TabVoice and wrecks the tablature. Before the staff puts the marks
% above it, after it puts them below.
songbookBeatMarks =
#(define-music-function (bars) (integer?)
   (let ((n (* 4 bars)))
     #{ \repeat unfold $n { s4^\markup { \with-color #red \bold "▮" } } #}))

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
