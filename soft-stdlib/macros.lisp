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

(setm* defmacro (fn* defmacro (name args &rest body)
    (block 
        (let body (cons 'block body))
        (quasi-quote (setm* `name (fn* `name `args `body))))))

(defmacro defun (name args &rest body)
    (block 
        (let body (cons 'block body))
        (quasi-quote (set* `name (fn* `name `args `body)))))

(defmacro letfun (name args &rest body)
    (block 
        (let body (cons 'block body))
        (quasi-quote (letrec `name (fn* `name `args `body)))))

(defmacro when (cond &rest body)
    (quasi-quote (if `cond `(cons 'block body) ())))

(defmacro unless (cond &rest body)
    (quasi-quote (if `cond () `(cons 'block body))))

(defun println (&rest body)
    (map* print body)
    (print "\n"))

(defmacro cond-internal (scrutinee &rest cases)
    (if (cons? cases)
        (block
            (let comp (head (head cases)))
            (let res  (head (tail (head cases))))
            (if (= 'else comp)
                res
                (block 
                    (let rest (list/concat (quasi-quote (cond-internal `scrutinee)) (tail cases)))
                    (quasi-quote
                        (if (= `scrutinee `comp)
                            `res
                            `rest)))))
        (throw "cond: no match")))

(defmacro cond (scrutinee &rest cases)
    (quasi-quote 
        (block
            (let __scrutinee `scrutinee)
            `(cons 'cond-internal (cons '__scrutinee cases)))))

(defmacro case-internal (&rest cases)
    (if (cons? cases)
        (block
            (let comp (head (head cases)))
            (let res  (head (tail (head cases))))
            (if (= 'else comp)
                res
                (block 
                    (let rest (cons 'case-internal (tail cases)))
                    (quasi-quote
                        (if `comp
                            `res
                            `rest)))))
        (throw "case: no match")))

(defmacro case (&rest cases)
    ~(block `(cons 'case-internal cases)))

(defmacro lambda (args &rest body)
    (quasi-quote (fn* unknown `args `(cons 'block body))))    

(defun match/matrix (cases) 
    (map* 
        (lambda (case) (cons (list (head case)) (tail case)))
        cases))
