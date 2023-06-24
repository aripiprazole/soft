(import "stdlib.lisp")

(defun foo (x)
  (throw "foo"))

(try* (foo 0)
  (err (err/print-stack err)))

(print (idx 0 (filter (fn* sla (x) (> x 1)) (list 1 2 3))))
