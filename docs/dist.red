Red [
	file: %dist.red
	description: {
		Draw a frequency plot of values visited, when randomly going up and down. A visual experiment for Autonomi Community Token non-deterministic magnitude pointers concept.
	}
	usage: {
		Run with Red's GUI flavor interpreter, e.g.: `red-view-10apr25-86494e40d dist.red`
	}
]

make-plot: function [] [
	
	x: 200
	
	xs: copy []
; 	loop 10000 [append xs (to integer! x: x + 5.5 - random/secure 10)]
	loop 10000 [append xs (to integer! x: x + 5.5 - random 10)]
; 	probe xs
	
	counters: copy #[]
	foreach x xs [counters/(x): (any [counters/(x) 0]) + 1]
;	probe counters
	counters: to block! counters
	sort/skip counters 2
;	probe counters
	
	draw-cmd: copy [
		pen red   line 200x0 200x10
		pen black line ]
	forall counters [
		point: make pair! reduce [counters/1 counters/2]
		append draw-cmd point
		counters: next counters
	]
;	probe draw-cmd
	draw-cmd
]

view [
	below
	base 400x60 draw make-plot
	base 400x60 draw make-plot
	base 400x60 draw make-plot
	base 400x60 draw make-plot
	base 400x60 draw make-plot
	base 400x60 draw make-plot
	base 400x60 draw make-plot
	base 400x60 draw make-plot
	base 400x60 draw make-plot
	base 400x60 draw make-plot
]
