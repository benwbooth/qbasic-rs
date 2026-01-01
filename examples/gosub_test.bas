CLS
PRINT "Testing GOSUB with text labels"
PRINT "Before GOSUB"
GOSUB TestSub
PRINT "After GOSUB"
PRINT "Done!"
INPUT "Press Enter to exit: ", x
END

TestSub:
    PRINT "Inside TestSub!"
RETURN
