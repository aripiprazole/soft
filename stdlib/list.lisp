(import "macros.lisp")

(defun map (f expr)
    (if (cons? expr)
        (cons (f (head expr)) (map f (tail expr)))
        expr))

(defun filter (f expr)
    (if (cons? expr)
        (if (f (head expr))
            (cons (head expr) (filter f (tail expr)))
            (filter f (tail expr)))
        expr))
