(import "macros.lisp")

(defun fib (num)
    (if (< num 2)
        num
        (+ (fib (- num 1)) (fib (- num 2)))))

(print (fib 10))