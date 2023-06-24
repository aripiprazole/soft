(set* map (fn* map (f expr)
    (if (cons? expr)
        (cons (f (head expr)) (map f (tail expr)))
        expr)))
