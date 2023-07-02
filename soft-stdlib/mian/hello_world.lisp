(script 
    (let counter 0)
    (let increment (fn () (set counter (+ counter 1))))
    (let decrement (fn () (set counter (- counter 1)))))

(h1 [] "Counter")
(button [(:on-click increment)] "Increment")
(p [] counter)
(button [(:on-click decrement)] "Decrement")

(style 
    (h1 (color "red"))
    (p (color "blue")))