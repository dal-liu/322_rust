PL_CLASS := L1
DST_PL_CLASS := S
COMPILER := ../target/release/$(PL_CLASS)
EXT_CLASS := $(PL_CLASS)
CC_CLASS := $(PL_CLASS)c

test: $(COMPILER)
	../scripts/test.sh $(EXT_CLASS) $(CC_CLASS) "tests"

clean:
	rm -fr *.out *.o `find tests -iname *.tmp`
	rm -fr *.$(DST_PL_CLASS)

.PHONY: test clean
