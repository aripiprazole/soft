(import "stdlib.lisp")

(defun repl ()
    (print "> ")
    (flush)
    (let line (read))
    (println (try* (map* eval (parse line "REPL")) (err err)))
    (repl))

(repl)