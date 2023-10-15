import React, {ReactElement} from "react";
import {useLocalStorage} from "@mantine/hooks";
import {ActionIcon, Box, Group, Space, Text} from "@mantine/core";
import {MdClose} from "react-icons/md";

interface IShowInfoPopup {
    readonly value: boolean
    readonly expiry: number
}

export function GeneralInfoPopup(): ReactElement {
    const [showInformationPopup, setInformationPopup] = useLocalStorage<IShowInfoPopup>({
        key: 'show-info-popup',
        defaultValue: {
            value: true,
            expiry: new Date().getTime() - 1000
        },
        getInitialValueInEffect: true,
    })

    if (showInformationPopup?.value || showInformationPopup?.expiry! < new Date().getTime()) {
        return <Box bg={"cyan"} my={'lg'} p={"md"}>
            <Group wrap={'nowrap'}>
                <Text style={{width: '100%'}} c={"#FFF"}>
                    Rezervările se fac până la ora 17 respectiv 19 pentru ziua respectivă. Max 8 jucători
                    pentru un
                    interval orar. Când știți că nu ajungeți, retrageți-vă pentru a lăsa loc liber altor
                    jucători. Spor la joc!</Text>

                <ActionIcon onClick={() => {
                    const daysInMilliseconds = 3 * 24 * 60 * 60 * 10000 // 3 days in milliseconds
                    const item: IShowInfoPopup = {
                        value: false,
                        expiry: new Date().getTime() + daysInMilliseconds
                    }

                    setInformationPopup(item)
                }} size={48}>
                    <MdClose size={24}/>
                </ActionIcon>
            </Group>
        </Box>
    }

    return <Space h={'lg'} />
}
