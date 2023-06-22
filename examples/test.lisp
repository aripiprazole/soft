(set* map (lambda* (f expr)
    (if (is-cons expr)
        (cons (f (head expr)) (map f (tail expr)))
        expr)))

(setm* quasi-quote (lambda* (expr)
    (if (is-cons expr)
        (if (eq 'unquote (head expr))
            (head (tail expr))
            (cons 'list (map quasi-quote expr)))
        (list 'quote expr))))

(setm* def-macro (lambda* (name args body)
    (quasi-quote
        (setm* ,name (lambda* ,args ,body)))))

(def-macro defn (name args body)
    (quasi-quote
        (set* ,name (lambda* ,args ,body))))

(defn fib (num)
    (if (< num 2)
        num
        (+ (fib (- num 1)) (fib (- num 2)))))

(print (fib 10))