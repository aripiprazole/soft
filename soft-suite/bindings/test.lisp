(let lib (ffi/open "./libtest.so"))
(let fun (ffi/get lib "teste" (int int string)))
(let applier (fn* teste (x y) (ffi/apply fun (list x y))))

(print (applier 1 2))