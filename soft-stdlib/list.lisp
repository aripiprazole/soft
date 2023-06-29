(import "macros.lisp")

(defun list/map (f expr)
    (if (cons? expr)
        (cons (f (head expr)) (list/map f (tail expr)))
        expr))


(defun list/concat (a b)
    (if (cons? a)
        (cons (head a) (list/concat (tail a) b))
        b))

(defun list/filter (f l)
    (if (cons? l)
        (if (f (head l))
            (cons (head l) (list/filter f (tail l)))
            (list/filter f (tail l)))
        ()))

(defun list/join (l)
    (if (cons? l)
        (list/concat (head l) (list/join (tail l)))
        ()))

(defun list/find (f l)
    (if (cons? l)
        (if (f (head l))
            (head l)
            (list/find f (tail l)))
        ()))

(defun list/length (l)
    (if (cons? l)
        (+ 1 (list/length (tail l)))
        0))

(defun list/enumerate (list)
    (letfun enumerate (list index)
        (if (cons? list) 
            (cons (cons index (cons (head list) ()))
                  (enumerate (tail list) (+ index 1)))
            ()))

    (enumerate list 0))

(defun list/fold (list init f)
    (if (cons? list)
        (f (head list) (list/fold (tail list) init f))
        init))

(defun list/ref (list index) 
  (if (= index 0) (head list) (list/ref (tail list) (- index 1))))

(defun list/is? (list) (if (= (type-of list) :cons) :true ()))

(defun list/zip (a b)
  (if (or (nil? a) (nil? b)) 
    ()
    (cons (pair (head a) (head b)) (list/zip (tail a) (tail b)))))
