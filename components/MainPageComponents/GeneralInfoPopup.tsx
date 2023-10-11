import React, {ReactElement} from "react";
import {useLocalStorage} from "@mantine/hooks";
import {ActionIcon, Group, Paper, Space, Text} from "@mantine/core";
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

    if (showInformationPopup.value || showInformationPopup.expiry < new Date().getTime()) {
        return <Paper shadow={"0"} p={"md"} sx={(theme) => ({
            backgroundColor: theme.colors.cyan[9],
            marginTop: theme.spacing.lg,
            marginBottom: theme.spacing.lg,
        })}>
            <Group noWrap={true}>
                <Text style={{width: '100%'}}>
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
        </Paper>
    }

    return <Space h={'lg'} />
}
