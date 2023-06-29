(set* map* (fn* map* (f expr)
    (if (cons? expr)
        (cons (f (head expr)) (map* f (tail expr)))
        expr)))

(setm* quasi-quote (fn* quasi-quote (expr)
    (if (cons? expr)
        (if (= 'unquote (head expr))
            (head (tail expr))
            (cons 'list (map* quasi-quote expr)))
        (list 'quote expr))))

(setm* defmacro (fn* defmacro (name args body)
    (quasi-quote
        (setm* `name (fn* `name `args `body)))))

(defmacro defun (name args &rest body)
    (block 
        (let body (cons 'block body))
        (quasi-quote (set* `name (fn* `name `args `body)))))

