import {Button, Card, Center, Group, Loader, Paper, Avatar, Stack, Text, Title} from "@mantine/core";
import React, {useEffect, useState} from "react";
import {NextRouter, useRouter} from "next/router";
import ReservationComponent from "../components/Reservation";
import {useAuth} from "../components/AuthProvider";
import {supabase} from "../utils/supabase_utils";
import {GameTable, Reservation, ReservationStatus} from "../types/wrapper";

interface IParams {
    gameTables: GameTable[]
}

async function signOut(router: NextRouter) {
    const {error} = await supabase.auth.signOut()

    if (error == null) {
        await router.push("/login")
    } else {
        console.log('Auth', error);
        console.log(error); // Failure
    }
}

export default function Profile(params: IParams) {
    const router = useRouter()
    const auth = useAuth()

    const [reservations, setReservations] = useState<Reservation[]>([])

    useEffect(() => {
        if (!auth.loading && auth.user == null)
            router.push('/login')
    }, [auth, router])

    async function fetchReservations() {
        if (auth.user == null)
            return;

        supabase.from<Reservation>('rezervari')
            .select('*')
            .eq("user_id", auth.user.id)
            .then(value => {
                if (value.data != null) {
                    setReservations(value.data)
                    console.log("Reservations updated")
                }
            })
    }

    useEffect(() => {
        fetchReservations();

        const subscription = supabase.from<Reservation>('rezervari')
            .on('INSERT', payload => {
                console.log(payload.new)
                if (payload.new.status == ReservationStatus.Approved) {
                    setReservations(prev => [...prev, payload.new])
                }
            })
            .on('UPDATE', payload => {
                setReservations(prev => [...prev.filter(value => value.id != payload.old.id), payload.new])
            })
            .on('DELETE', payload => {
                setReservations(prev => prev.filter(value => value.id != payload.old.id))
            })
            .subscribe()

        return () => {
            subscription?.unsubscribe()
        }
    }, [auth])

    if (auth.loading)
        return <Center> <Loader/> </Center>;

    if (auth.user == null)
        return (<></>)

    return (<>
        <Paper shadow={"md"} p={"xl"} sx={(theme) => ({
            backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[4] : theme.colors.gray[1],
            margin: theme.spacing.md
        })}>

            <Group position={"apart"}>
                <Group noWrap={true}>
                    {auth.profile != null &&
                        <>
                            <Avatar src={`https://ui-avatars.com/api/?name=${auth.profile.name}&background=random`}
                                    radius={"md"}/>

                            <Text size="md">{auth.profile.name}</Text>
                        </>
                    }
                </Group>

                <Button variant={"filled"} color={'red'} onClick={() => signOut(router)}>Sign out</Button>
            </Group>

        </Paper>

        <Stack p={"xl"}>
            <Title order={2}>Rezervările tale:</Title>

            {reservations.length == 0 &&
                <Text size={"lg"}>Nu ați făcut nici o rezervare</Text>
            }

            {reservations.map((reservation) => (
                <Card key={reservation.id}>
                    {ReservationComponent(
                        reservation,
                        params.gameTables.find((element) => element.id == reservation.table_id),
                        true,
                        async () => {
                            const newData = {
                                ...reservation,
                                status: ReservationStatus.Cancelled
                            }

                            console.log("Hello")

                            await supabase.from<Reservation>('rezervari')
                                .update(newData, {returning: 'minimal'})

                            console.log("There")
                        }
                    )}
                </Card>
            ))}
        </Stack>
    </>)
}

export async function getStaticProps({}) {
    const {data: gameTables} = await supabase.from<GameTable>('mese').select('*')

    const props: IParams = {
        gameTables
    }

    return {
        props: props
    }
}
